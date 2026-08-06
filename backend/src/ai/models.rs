use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, Document, oid::ObjectId};
use serde::{Deserialize, Serialize};

// API response shapes (ai::service::AiService). `cached` isn't in
// docs/api.md's field list but is cheap to include and directly useful: it's
// what the service-layer tests assert on to prove a cache hit skipped the
// Gemini call, and it doubles as a signal the frontend can show later ("from
// cache" vs. freshly generated) without adding any new capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketSummaryResponse {
    pub summary: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAnalysisResponse {
    pub severity_prediction: String,
    pub suggested_fix: String,
    pub classification: String,
    pub cached: bool,
}

// One document per ticket, upserted in place by summarize/analyze
// (ai::repository::AiRepository::upsert_summary / upsert_analysis) rather than
// inserted fresh each call — docs/database.md's "tickets -> ai_ticket_insights
// (1-to-1 or 1-to-many over time)" resolves to 1-to-1 here, with "over time"
// being this document's fields filling in / refreshing as each action runs.
//
// summary_source_updated_at / analysis_source_updated_at each snapshot the
// ticket's content_updated_at (ticket::models::Ticket — bumped only by
// title/description/priority edits, NOT status) at the moment that field
// group was last generated, and are tracked separately rather than as one
// shared fingerprint: summarize and analyze are independent calls, so an
// edit can invalidate one without the other having ever run, or vice versa.
// A field group is fresh iff its own value is present AND its source
// timestamp still matches the ticket's current content_updated_at (see
// is_summary_fresh / is_analysis_fresh below) — this is the mechanism behind
// docs/ai-integration.md's "cached per ticket... if ticket does not change,
// AI is not re-run". Deliberately keyed off content_updated_at rather than
// the ticket's plain updated_at: closing/reopening a ticket shouldn't throw
// away a cached summary/analysis, since the AI never reads status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTicketInsight {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub summary: Option<String>,
    pub summary_source_updated_at: Option<BsonDateTime>,
    pub severity_prediction: Option<String>,
    pub suggested_fix: Option<String>,
    pub classification: Option<String>,
    pub analysis_source_updated_at: Option<BsonDateTime>,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

impl AiTicketInsight {
    pub fn is_summary_fresh(&self, ticket_content_updated_at: BsonDateTime) -> bool {
        self.summary.is_some()
            && self.summary_source_updated_at == Some(ticket_content_updated_at)
    }

    pub fn is_analysis_fresh(&self, ticket_content_updated_at: BsonDateTime) -> bool {
        self.severity_prediction.is_some()
            && self.suggested_fix.is_some()
            && self.classification.is_some()
            && self.analysis_source_updated_at == Some(ticket_content_updated_at)
    }
}

// docs/api.md's "POST /ai/groups/{id}/report" returns "group-wide analytics,
// ticket trends, workload distribution". There's no assignee/owner concept
// in the ticket model (only created_by, status, priority — see
// ticket::models::Ticket), so "workload distribution" is read here as ticket
// volume distributed across priority/status rather than per-person load;
// `narrative` is where Gemini synthesizes trends/workload commentary in
// prose from these hard numbers (see AiProvider::narrate_report — the model
// narrates, it doesn't count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityBreakdown {
    pub low: i64,
    pub high: i64,
    pub critical: i64,
}

// Backing shape for AiGroupReport::report_data. Stored as a plain Document in
// Mongo (docs/database.md doesn't pin report_data's schema) but always
// round-tripped through this struct via bson::to_document/from_document, so
// nothing hand-rolls doc! key access for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportData {
    pub total_tickets: i64,
    pub open_tickets: i64,
    pub closed_tickets: i64,
    pub priority_breakdown: PriorityBreakdown,
    pub recent_tickets_7d: i64,
    pub narrative: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupReportResponse {
    #[serde(flatten)]
    pub data: ReportData,
    pub generated_at: DateTime<Utc>,
    pub cached: bool,
}

// Aggregated group-wide analytics document (docs/api.md, "POST
// /ai/groups/{id}/report"). One new document per generation (unlike
// AiTicketInsight's single upserted-in-place doc) — docs/database.md:
// "groups -> ai_group_reports (1-to-many)" — so find_latest_report always
// reads the most recent one and older reports remain as a history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGroupReport {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub report_data: Document,
    pub generated_at: BsonDateTime,
    pub generated_by: ObjectId,
}

impl AiGroupReport {
    // docs/ai-integration.md: "Group reports are cached for a period of
    // time" — unlike ticket insights (invalidated by a content change),
    // reports are cheap-but-imprecise to invalidate exactly (would mean
    // tracking every ticket's content_updated_at across the whole group), so
    // a fixed time window is used instead (confirmed with user: 1 hour).
    pub fn is_fresh(&self, now: BsonDateTime, ttl_millis: i64) -> bool {
        now.timestamp_millis() - self.generated_at.timestamp_millis() < ttl_millis
    }
}

// "assistant", not "ai" or "gemini": keeps the field provider-agnostic in
// case the backing model ever changes, same reasoning as AiProvider being a
// trait rather than a concrete GeminiClient type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    User,
    Assistant,
}

// One document per message, unlike AiTicketInsight's single upserted-in-place
// doc — a conversation is inherently a sequence, not a single latest value.
// Structurally close to comment::models::Comment (same group_id/ticket_id
// scoping, same insert-only, oldest-first list shape) rather than anything
// else in this module. user_id is None for an assistant message and Some for
// a user message — see AiRepository::count_recent_user_messages, which
// filters on both role and user_id being present to count one user's own
// messages for the rate limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub role: ChatRole,
    pub user_id: Option<ObjectId>,
    pub content: String,
    pub created_at: BsonDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendChatMessageRequest {
    pub message: String,
}

// user_name is enriched the same way CommentResponse's is (UserService
// lookup) — None for an assistant message, same as user_id on the underlying
// ChatMessage. The ticket's chat is a shared, group-visible thread like
// comments, not a private per-user conversation, so who asked matters when
// more than one member has used it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub id: String,
    pub role: ChatRole,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// Returns both messages from a single POST rather than just the assistant's
// reply: the frontend would otherwise have to fabricate an id/timestamp for
// the user's own message to render it immediately, and that fabricated id
// would never match the persisted one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendChatMessageResponse {
    pub user_message: ChatMessageResponse,
    pub assistant_message: ChatMessageResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insight_at(ts: BsonDateTime) -> AiTicketInsight {
        AiTicketInsight {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            ticket_id: ObjectId::new(),
            summary: Some("summary text".to_string()),
            summary_source_updated_at: Some(ts),
            severity_prediction: Some("high".to_string()),
            suggested_fix: Some("fix text".to_string()),
            classification: Some("bug".to_string()),
            analysis_source_updated_at: Some(ts),
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn summary_fresh_when_timestamp_matches() {
        let ts = BsonDateTime::now();
        let insight = insight_at(ts);
        assert!(insight.is_summary_fresh(ts));
        assert!(insight.is_analysis_fresh(ts));
    }

    #[test]
    fn summary_stale_when_ticket_changed_since() {
        let generated_at = BsonDateTime::from_millis(1_000);
        let ticket_now = BsonDateTime::from_millis(2_000);
        let insight = insight_at(generated_at);
        assert!(!insight.is_summary_fresh(ticket_now));
        assert!(!insight.is_analysis_fresh(ticket_now));
    }

    #[test]
    fn summary_not_fresh_when_never_generated() {
        let ts = BsonDateTime::now();
        let mut insight = insight_at(ts);
        insight.summary = None;
        insight.summary_source_updated_at = None;
        assert!(!insight.is_summary_fresh(ts));
        // Analysis fields are independent of summary being absent.
        assert!(insight.is_analysis_fresh(ts));
    }

    #[test]
    fn analysis_not_fresh_when_only_partially_generated() {
        let ts = BsonDateTime::now();
        let mut insight = insight_at(ts);
        insight.suggested_fix = None;
        assert!(!insight.is_analysis_fresh(ts));
    }

    fn report_at(generated_at: BsonDateTime) -> AiGroupReport {
        AiGroupReport {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            report_data: Document::new(),
            generated_at,
            generated_by: ObjectId::new(),
        }
    }

    const ONE_HOUR_MILLIS: i64 = 60 * 60 * 1000;

    #[test]
    fn report_fresh_within_ttl() {
        let generated_at = BsonDateTime::from_millis(0);
        let now = BsonDateTime::from_millis(ONE_HOUR_MILLIS - 1);
        assert!(report_at(generated_at).is_fresh(now, ONE_HOUR_MILLIS));
    }

    #[test]
    fn report_stale_after_ttl() {
        let generated_at = BsonDateTime::from_millis(0);
        let now = BsonDateTime::from_millis(ONE_HOUR_MILLIS);
        assert!(!report_at(generated_at).is_fresh(now, ONE_HOUR_MILLIS));
    }

    #[test]
    fn report_fresh_at_generation_instant() {
        let ts = BsonDateTime::now();
        assert!(report_at(ts).is_fresh(ts, ONE_HOUR_MILLIS));
    }
}
