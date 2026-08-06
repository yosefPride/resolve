use chrono::{DateTime, Utc};
use mongodb::{
    Database,
    bson::{self, DateTime as BsonDateTime, oid::ObjectId},
};

use crate::ai::client::{AiProvider, GeminiClient};
use crate::ai::models::{
    ChatMessage, ChatMessageResponse, ChatRole, GroupReportResponse, PriorityBreakdown, ReportData,
    SendChatMessageResponse, TicketAnalysisResponse, TicketSummaryResponse,
};
use crate::ai::repository::AiRepository;
use crate::config::Config;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::models::{Ticket, TicketStatus};
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

// Confirmed with user: reuse a generated report for up to an hour before
// regenerating on request (docs/ai-integration.md: "Group reports are
// cached for a period of time").
const REPORT_TTL_MILLIS: i64 = 60 * 60 * 1000;
const RECENT_WINDOW_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;

// Confirmed with user: 10 messages per hour, scoped per user (see
// AiRepository::count_recent_user_messages) rather than per ticket — a
// guardrail against one person hammering the endpoint, not a
// per-conversation quota.
const CHAT_RATE_LIMIT: u64 = 10;
const CHAT_RATE_WINDOW_MILLIS: i64 = 60 * 60 * 1000;
// How many prior messages get fed back to Gemini as context on each turn.
// Bounds prompt size/cost regardless of how long a conversation has run —
// the full history still lists in the API response either way, this only
// caps what the model sees.
const CHAT_HISTORY_LIMIT: usize = 20;

// Generic over the provider (default GeminiClient) rather than a concrete
// GeminiClient field: production call sites (AiService::new) are unaffected
// by the default type parameter, while tests can build an
// AiService<FakeProvider> via with_provider to exercise the cache/RBAC logic
// below without ever touching the network — same reasoning as AiProvider's
// doc comment on why it exists at all.
pub struct AiService<P: AiProvider = GeminiClient> {
    repo: AiRepository,
    ticket_repo: TicketRepository,
    user_service: UserService,
    rbac: RbacService,
    provider: P,
}

impl AiService<GeminiClient> {
    // Fails per-request rather than at startup — see Config::gemini_api_key's
    // doc comment on why a missing key doesn't block the whole backend from
    // booting.
    pub fn new(db: &Database, config: &Config) -> Result<Self, ApiError> {
        let api_key = config.gemini_api_key.clone().ok_or(ApiError::Internal)?;
        Ok(Self::with_provider(db, GeminiClient::new(api_key)))
    }
}

impl<P: AiProvider> AiService<P> {
    pub fn with_provider(db: &Database, provider: P) -> Self {
        Self {
            repo: AiRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
            provider,
        }
    }

    // docs/api.md: "POST /ai/groups/{id}/tickets/{ticket_id}/summarize —
    // Group-scoped (member of {id} required)".
    pub async fn summarize_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<TicketSummaryResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let cached = self.repo.find_insight(group_id, ticket_id).await?;
        if let Some(insight) = &cached
            && insight.is_summary_fresh(ticket.content_updated_at)
        {
            return Ok(TicketSummaryResponse {
                summary: insight
                    .summary
                    .clone()
                    .expect("is_summary_fresh implies Some"),
                cached: true,
            });
        }

        let summary = self
            .provider
            .summarize(&ticket.title, &ticket.description)
            .await?;
        self.repo
            .upsert_summary(group_id, ticket_id, &summary, ticket.content_updated_at)
            .await?;
        Ok(TicketSummaryResponse {
            summary,
            cached: false,
        })
    }

    // docs/api.md: "POST /ai/groups/{id}/tickets/{ticket_id}/analyze —
    // Group-scoped (member of {id} required)".
    pub async fn analyze_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<TicketAnalysisResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let cached = self.repo.find_insight(group_id, ticket_id).await?;
        if let Some(insight) = &cached
            && insight.is_analysis_fresh(ticket.content_updated_at)
        {
            return Ok(TicketAnalysisResponse {
                severity_prediction: insight
                    .severity_prediction
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                suggested_fix: insight
                    .suggested_fix
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                classification: insight
                    .classification
                    .clone()
                    .expect("is_analysis_fresh implies Some"),
                cached: true,
            });
        }

        let analysis = self
            .provider
            .analyze(&ticket.title, &ticket.description)
            .await?;
        self.repo
            .upsert_analysis(
                group_id,
                ticket_id,
                &analysis.severity_prediction,
                &analysis.suggested_fix,
                &analysis.classification,
                ticket.content_updated_at,
            )
            .await?;
        Ok(TicketAnalysisResponse {
            severity_prediction: analysis.severity_prediction,
            suggested_fix: analysis.suggested_fix,
            classification: analysis.classification,
            cached: false,
        })
    }

    // docs/api.md: "POST /ai/groups/{id}/report — Group Admin only".
    pub async fn generate_group_report(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<GroupReportResponse, ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;

        let now = BsonDateTime::now();
        if let Some(report) = self.repo.find_latest_report(group_id).await?
            && report.is_fresh(now, REPORT_TTL_MILLIS)
        {
            let data: ReportData =
                bson::from_document(report.report_data).map_err(|_| ApiError::Internal)?;
            return Ok(GroupReportResponse {
                data,
                generated_at: to_chrono(report.generated_at),
                cached: true,
            });
        }

        let tickets = self
            .ticket_repo
            .list_by_group(group_id, None, None, None)
            .await?;
        let stats = aggregate_stats(&tickets, now);
        let narrative = self.provider.narrate_report(&stats_summary(&stats)).await?;
        let data = ReportData {
            total_tickets: stats.total_tickets,
            open_tickets: stats.open_tickets,
            closed_tickets: stats.closed_tickets,
            priority_breakdown: stats.priority_breakdown,
            recent_tickets_7d: stats.recent_tickets_7d,
            narrative,
        };

        let report_data =
            bson::to_document(&data).expect("ReportData always serializes to a Document");
        // Use the inserted document's own generated_at (set inside
        // insert_report via a fresh BsonDateTime::now()), not the `now`
        // captured above — that one predates the narrate_report network
        // call, so it would otherwise report an earlier timestamp than
        // what's actually persisted.
        let inserted = self
            .repo
            .insert_report(group_id, report_data, user_id)
            .await?;

        Ok(GroupReportResponse {
            data,
            generated_at: to_chrono(inserted.generated_at),
            cached: false,
        })
    }

    // Any group member may chat about a ticket, same as summarize/analyze —
    // no admin-only gate. Unlike those two, this is never cacheable: each
    // message is a genuinely new request, there's no "fresh iff unchanged"
    // check to make. Message format validation (empty/over-length) lives in
    // the handler only, same as comment content — see
    // comment_handlers::validate_create's doc comment for why the split is
    // there and not duplicated here.
    pub async fn send_chat_message(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        message: String,
    ) -> Result<SendChatMessageResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let since = BsonDateTime::from_millis(
            BsonDateTime::now().timestamp_millis() - CHAT_RATE_WINDOW_MILLIS,
        );
        let recent_count = self.repo.count_recent_user_messages(user_id, since).await?;
        if recent_count >= CHAT_RATE_LIMIT {
            return Err(ApiError::RateLimited(format!(
                "chat message limit reached ({CHAT_RATE_LIMIT} per hour) — try again later"
            )));
        }

        let history = self.repo.list_chat_messages(group_id, ticket_id).await?;
        let transcript = build_transcript(&history);
        let reply = self
            .provider
            .chat(&ticket.title, &ticket.description, &transcript, &message)
            .await?;

        let user_message = self
            .repo
            .insert_chat_message(group_id, ticket_id, ChatRole::User, Some(user_id), &message)
            .await?;
        let assistant_message = self
            .repo
            .insert_chat_message(group_id, ticket_id, ChatRole::Assistant, None, &reply)
            .await?;

        Ok(SendChatMessageResponse {
            user_message: self.enrich_chat_message(user_message).await?,
            assistant_message: self.enrich_chat_message(assistant_message).await?,
        })
    }

    // Whole thread, oldest-first, no pagination — same shape and reasoning as
    // CommentService::list_comments (docs/implementation/backend/08-ai.md).
    pub async fn list_chat_messages(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<ChatMessageResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let messages = self.repo.list_chat_messages(group_id, ticket_id).await?;
        let mut result = Vec::with_capacity(messages.len());
        for message in messages {
            result.push(self.enrich_chat_message(message).await?);
        }
        Ok(result)
    }

    // "New chat": clears the ticket's one ongoing conversation. Any member
    // may do this, same as any member may send a message — the thread is
    // shared group-visible content like comments, not a private one owned by
    // whoever started it, so there's no per-author ownership check here the
    // way comment deletion has (RbacService::require_owner_or_group_admin).
    pub async fn clear_chat(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<(), ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        self.repo.clear_chat_messages(group_id, ticket_id).await?;
        Ok(())
    }

    // ChatMessageResponse needs the author's display name, which ChatMessage
    // doesn't carry — mirrors CommentService::enrich_comment. None for an
    // assistant message (no user_id to look up).
    async fn enrich_chat_message(
        &self,
        message: ChatMessage,
    ) -> Result<ChatMessageResponse, ApiError> {
        let user_name = match message.user_id {
            Some(uid) => self.user_service.find_by_id(uid).await?.map(|u| u.name),
            None => None,
        };
        Ok(ChatMessageResponse {
            id: message.id.map(|id| id.to_hex()).unwrap_or_default(),
            role: message.role,
            user_id: message.user_id.map(|id| id.to_hex()),
            user_name,
            content: message.content,
            created_at: to_chrono(message.created_at),
        })
    }
}

// Last CHAT_HISTORY_LIMIT messages, oldest-first, rendered as plain text for
// the prompt — kept as a free function so it's unit-testable without a
// provider or DB (same reasoning as aggregate_stats/stats_summary below).
fn build_transcript(history: &[ChatMessage]) -> String {
    let start = history.len().saturating_sub(CHAT_HISTORY_LIMIT);
    history[start..]
        .iter()
        .map(|message| {
            let speaker = match message.role {
                ChatRole::User => "User",
                ChatRole::Assistant => "Assistant",
            };
            format!("{speaker}: {}", message.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_chrono(ts: BsonDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ts.timestamp_millis()).unwrap_or_default()
}

// Intermediate aggregate, pre-narrative — kept separate from ReportData so
// aggregate_stats stays a pure function (no provider call inside it) and can
// be unit-tested directly against a plain Vec<Ticket>, no DB or network.
struct Stats {
    total_tickets: i64,
    open_tickets: i64,
    closed_tickets: i64,
    priority_breakdown: PriorityBreakdown,
    recent_tickets_7d: i64,
}

fn aggregate_stats(tickets: &[Ticket], now: BsonDateTime) -> Stats {
    let mut open = 0i64;
    let mut closed = 0i64;
    let mut low = 0i64;
    let mut high = 0i64;
    let mut critical = 0i64;
    let mut recent = 0i64;

    for ticket in tickets {
        match ticket.status {
            TicketStatus::Open => open += 1,
            TicketStatus::Closed => closed += 1,
        }
        match ticket.priority {
            crate::ticket::models::TicketPriority::Low => low += 1,
            crate::ticket::models::TicketPriority::High => high += 1,
            crate::ticket::models::TicketPriority::Critical => critical += 1,
        }
        if now.timestamp_millis() - ticket.created_at.timestamp_millis() < RECENT_WINDOW_MILLIS {
            recent += 1;
        }
    }

    Stats {
        total_tickets: tickets.len() as i64,
        open_tickets: open,
        closed_tickets: closed,
        priority_breakdown: PriorityBreakdown {
            low,
            high,
            critical,
        },
        recent_tickets_7d: recent,
    }
}

fn stats_summary(stats: &Stats) -> String {
    format!(
        "Total tickets: {}. Open: {}. Closed: {}. Priority breakdown — low: {}, high: {}, critical: {}. Created in the last 7 days: {}.",
        stats.total_tickets,
        stats.open_tickets,
        stats.closed_tickets,
        stats.priority_breakdown.low,
        stats.priority_breakdown.high,
        stats.priority_breakdown.critical,
        stats.recent_tickets_7d,
    )
}

#[cfg(test)]
mod tests {
    use mongodb::bson::oid::ObjectId;

    use super::*;
    use crate::ticket::models::{TicketPriority, TicketStatus};

    fn ticket(status: TicketStatus, priority: TicketPriority, created_at: BsonDateTime) -> Ticket {
        Ticket {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            ticket_number: 1,
            title: "t".to_string(),
            description: "d".to_string(),
            status,
            priority,
            created_by: ObjectId::new(),
            created_at,
            updated_at: created_at,
            content_updated_at: created_at,
        }
    }

    #[test]
    fn aggregate_stats_counts_status_and_priority() {
        let now = BsonDateTime::now();
        let tickets = vec![
            ticket(TicketStatus::Open, TicketPriority::Low, now),
            ticket(TicketStatus::Open, TicketPriority::High, now),
            ticket(TicketStatus::Closed, TicketPriority::Critical, now),
        ];
        let stats = aggregate_stats(&tickets, now);
        assert_eq!(stats.total_tickets, 3);
        assert_eq!(stats.open_tickets, 2);
        assert_eq!(stats.closed_tickets, 1);
        assert_eq!(
            stats.priority_breakdown,
            PriorityBreakdown {
                low: 1,
                high: 1,
                critical: 1
            }
        );
    }

    #[test]
    fn aggregate_stats_counts_recent_tickets_within_7_days() {
        let now = BsonDateTime::from_millis(RECENT_WINDOW_MILLIS * 2);
        let recent = BsonDateTime::from_millis(RECENT_WINDOW_MILLIS * 2 - 1_000);
        let old = BsonDateTime::from_millis(0);
        let tickets = vec![
            ticket(TicketStatus::Open, TicketPriority::Low, recent),
            ticket(TicketStatus::Open, TicketPriority::Low, old),
        ];
        let stats = aggregate_stats(&tickets, now);
        assert_eq!(stats.recent_tickets_7d, 1);
    }

    #[test]
    fn aggregate_stats_on_empty_group_is_all_zero() {
        let now = BsonDateTime::now();
        let stats = aggregate_stats(&[], now);
        assert_eq!(stats.total_tickets, 0);
        assert_eq!(stats.recent_tickets_7d, 0);
    }

    fn message(role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            ticket_id: ObjectId::new(),
            role,
            user_id: match role {
                ChatRole::User => Some(ObjectId::new()),
                ChatRole::Assistant => None,
            },
            content: content.to_string(),
            created_at: BsonDateTime::now(),
        }
    }

    #[test]
    fn build_transcript_on_empty_history_is_empty_string() {
        assert_eq!(build_transcript(&[]), "");
    }

    #[test]
    fn build_transcript_renders_speaker_and_content_oldest_first() {
        let history = vec![
            message(ChatRole::User, "hi"),
            message(ChatRole::Assistant, "hello"),
        ];
        assert_eq!(build_transcript(&history), "User: hi\nAssistant: hello");
    }

    #[test]
    fn build_transcript_caps_to_last_chat_history_limit_messages() {
        let history: Vec<ChatMessage> = (0..CHAT_HISTORY_LIMIT + 5)
            .map(|i| message(ChatRole::User, &i.to_string()))
            .collect();
        let transcript = build_transcript(&history);
        let lines: Vec<&str> = transcript.lines().collect();
        assert_eq!(lines.len(), CHAT_HISTORY_LIMIT);
        // Oldest of the kept messages is index 5 (the first 5 were dropped),
        // newest is the very last one appended.
        assert_eq!(lines.first(), Some(&"User: 5"));
        assert_eq!(
            *lines.last().unwrap(),
            format!("User: {}", CHAT_HISTORY_LIMIT + 4)
        );
    }
}
