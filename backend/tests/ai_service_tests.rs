use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mongodb::bson::{DateTime as BsonDateTime, Document, doc, oid::ObjectId};
use resolve::ai::client::{AiProvider, AnalysisResult};
use resolve::ai::models::ChatRole;
use resolve::ai::service::AiService;
use resolve::errors::ApiError;
use resolve::group::models::{CreateGroupInput, Role};
use resolve::group::repository::GroupRepository;
use resolve::ticket::models::{CreateTicketInput, TicketPriority};
use resolve::ticket::repository::TicketRepository;

mod support;

// Records how many times each method was called and hands back canned
// results, so tests can assert "the network was (not) hit" without any real
// HTTP — the whole point of AiProvider being a trait (see ai::client's doc
// comment on it).
#[derive(Clone)]
struct FakeProvider {
    summarize_calls: Arc<AtomicUsize>,
    analyze_calls: Arc<AtomicUsize>,
    narrate_calls: Arc<AtomicUsize>,
    chat_calls: Arc<AtomicUsize>,
    summary: Arc<Mutex<String>>,
    analysis: Arc<Mutex<AnalysisResult>>,
    narrative: Arc<Mutex<String>>,
    chat_reply: Arc<Mutex<String>>,
    // Captures the transcript seen on the most recent chat() call, so tests
    // can assert prior history was actually threaded through rather than
    // just checking the reply text.
    last_chat_transcript: Arc<Mutex<String>>,
}

impl FakeProvider {
    fn new(summary: &str, analysis: AnalysisResult) -> Self {
        Self {
            summarize_calls: Arc::new(AtomicUsize::new(0)),
            analyze_calls: Arc::new(AtomicUsize::new(0)),
            narrate_calls: Arc::new(AtomicUsize::new(0)),
            chat_calls: Arc::new(AtomicUsize::new(0)),
            summary: Arc::new(Mutex::new(summary.to_string())),
            analysis: Arc::new(Mutex::new(analysis)),
            narrative: Arc::new(Mutex::new("a narrative".to_string())),
            chat_reply: Arc::new(Mutex::new("a chat reply".to_string())),
            last_chat_transcript: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl AiProvider for FakeProvider {
    fn summarize(
        &self,
        _title: &str,
        _description: &str,
    ) -> impl Future<Output = Result<String, ApiError>> + Send {
        self.summarize_calls.fetch_add(1, Ordering::SeqCst);
        let summary = self.summary.lock().unwrap().clone();
        async move { Ok(summary) }
    }

    fn analyze(
        &self,
        _title: &str,
        _description: &str,
    ) -> impl Future<Output = Result<AnalysisResult, ApiError>> + Send {
        self.analyze_calls.fetch_add(1, Ordering::SeqCst);
        let analysis = self.analysis.lock().unwrap().clone();
        async move { Ok(analysis) }
    }

    fn narrate_report(
        &self,
        _stats_summary: &str,
    ) -> impl Future<Output = Result<String, ApiError>> + Send {
        self.narrate_calls.fetch_add(1, Ordering::SeqCst);
        let narrative = self.narrative.lock().unwrap().clone();
        async move { Ok(narrative) }
    }

    fn chat(
        &self,
        _title: &str,
        _description: &str,
        transcript: &str,
        _message: &str,
    ) -> impl Future<Output = Result<String, ApiError>> + Send {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_chat_transcript.lock().unwrap() = transcript.to_string();
        let reply = self.chat_reply.lock().unwrap().clone();
        async move { Ok(reply) }
    }
}

fn default_analysis() -> AnalysisResult {
    AnalysisResult {
        severity_prediction: "high".to_string(),
        suggested_fix: "restart the service".to_string(),
        classification: "bug".to_string(),
    }
}

// Returns (db, group_id, member_id, ticket_id): a group with one Contributor
// member and one seeded ticket, ready for summarize_ticket/analyze_ticket
// calls.
async fn setup() -> (mongodb::Database, ObjectId, ObjectId, ObjectId) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "ai_ticket_insights",
        "ai_group_reports",
        "ai_chat_messages",
        "groups",
        "group_members",
        "tickets",
        "counters",
    ] {
        db.collection::<Document>(collection)
            .drop()
            .await
            .unwrap_or_else(|_| panic!("failed to drop {collection} collection"));
    }

    let group_repo = GroupRepository::new(&db);
    let ticket_repo = TicketRepository::new(&db);

    let owner_id = ObjectId::new();
    let group = group_repo
        .create_group(CreateGroupInput {
            name: "Test Group".to_string(),
            owner_id,
        })
        .await
        .expect("create_group failed");
    let group_id = group.id.expect("created group has an id");

    let member_id = ObjectId::new();
    group_repo
        .insert_member(group_id, member_id, Role::Contributor)
        .await
        .expect("insert_member failed");

    let ticket_number = ticket_repo
        .next_ticket_number(group_id)
        .await
        .expect("next_ticket_number failed");
    let ticket = ticket_repo
        .insert_ticket(CreateTicketInput {
            group_id,
            ticket_number,
            title: "Login button unresponsive".to_string(),
            description: "Tapping login on mobile Safari does nothing.".to_string(),
            priority: TicketPriority::High,
            created_by: owner_id,
        })
        .await
        .expect("insert_ticket failed");
    let ticket_id = ticket.id.expect("created ticket has an id");

    (db, group_id, member_id, ticket_id)
}

// Returns (db, owner_id, member_id, group_id): a group with one Group Admin
// (owner_id) and one Contributor (member_id), seeded with three tickets —
// open/low, open/high, closed/critical — so report tests can assert exact
// counts.
async fn setup_for_report() -> (mongodb::Database, ObjectId, ObjectId, ObjectId) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "ai_ticket_insights",
        "ai_group_reports",
        "ai_chat_messages",
        "groups",
        "group_members",
        "tickets",
        "counters",
    ] {
        db.collection::<Document>(collection)
            .drop()
            .await
            .unwrap_or_else(|_| panic!("failed to drop {collection} collection"));
    }

    let group_repo = GroupRepository::new(&db);
    let ticket_repo = TicketRepository::new(&db);

    let owner_id = ObjectId::new();
    let group = group_repo
        .create_group(CreateGroupInput {
            name: "Report Test Group".to_string(),
            owner_id,
        })
        .await
        .expect("create_group failed");
    let group_id = group.id.expect("created group has an id");

    // GroupRepository::create_group only inserts the group document — the
    // creator's own membership row is a separate insert_member call that
    // GroupService::create_group normally does alongside it (see its doc
    // comment). Going through the repo directly here means that has to be
    // done explicitly, or owner_id has no membership at all and
    // require_group_admin rejects it.
    group_repo
        .insert_member(group_id, owner_id, Role::GroupAdmin)
        .await
        .expect("insert_member (owner) failed");

    let member_id = ObjectId::new();
    group_repo
        .insert_member(group_id, member_id, Role::Contributor)
        .await
        .expect("insert_member failed");

    for (priority, close) in [
        (TicketPriority::Low, false),
        (TicketPriority::High, false),
        (TicketPriority::Critical, true),
    ] {
        let ticket_number = ticket_repo
            .next_ticket_number(group_id)
            .await
            .expect("next_ticket_number failed");
        let ticket = ticket_repo
            .insert_ticket(CreateTicketInput {
                group_id,
                ticket_number,
                title: "a ticket".to_string(),
                description: "a description".to_string(),
                priority,
                created_by: owner_id,
            })
            .await
            .expect("insert_ticket failed");
        if close {
            ticket_repo
                .update_ticket(
                    group_id,
                    ticket.id.unwrap(),
                    doc! { "status": "closed", "updated_at": BsonDateTime::now() },
                )
                .await
                .expect("update_ticket failed");
        }
    }

    (db, owner_id, member_id, group_id)
}

// 1. First summarize call has nothing cached, so it calls the provider.
#[test]
fn test_summarize_ticket_calls_provider_on_first_request() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("a concise summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        let response = service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("summarize_ticket failed");

        assert_eq!(response.summary, "a concise summary");
        assert!(!response.cached);
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 1);
    });
}

// 2. A second call for the same unchanged ticket is served from cache.
#[test]
fn test_summarize_ticket_uses_cache_on_second_request() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("a concise summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("first summarize_ticket failed");
        let second = service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("second summarize_ticket failed");

        assert_eq!(second.summary, "a concise summary");
        assert!(second.cached);
        // Still 1: the second call was served entirely from the cache.
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 1);
    });
}

// 3. Editing the ticket invalidates the cached summary, so the next call
// re-invokes the provider.
#[test]
fn test_summarize_ticket_recalls_provider_after_ticket_edit() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("first summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("first summarize_ticket failed");

        // Mongo's Date type has millisecond resolution, and this test (with
        // a fake, non-network provider) can otherwise run fast enough for
        // the edit's content_updated_at to collide with the cached
        // fingerprint — same reasoning as ai_repository_tests's
        // report-ordering test. A real Gemini round-trip takes far longer
        // than 5ms, so this can't happen in production.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // TicketRepository::update_ticket doesn't bump updated_at/
        // content_updated_at itself — that's TicketService::update_ticket's
        // job, which decides per-field whether content_updated_at moves
        // (ticket::models::Ticket's doc comment: title/description/priority
        // only, not status). Going through the repo directly here (skipping
        // RBAC and other ticket-service concerns this test doesn't care
        // about) means both must be set explicitly, the same way the service
        // does it for a content-bearing edit.
        let ticket_repo = TicketRepository::new(&db);
        let now = BsonDateTime::now();
        ticket_repo
            .update_ticket(
                group_id,
                ticket_id,
                doc! {
                    "description": "Now also broken on desktop Chrome.",
                    "updated_at": now,
                    "content_updated_at": now,
                },
            )
            .await
            .expect("update_ticket failed");

        let response = service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("second summarize_ticket failed");

        assert!(!response.cached);
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 2);
    });
}

// 4. A status-only change (closing the ticket) does NOT invalidate the
// cache: the AI only ever reads title/description, so content_updated_at
// (and therefore the cached summary) is untouched by a status flip.
#[test]
fn test_summarize_ticket_stays_cached_after_status_only_change() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("a concise summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("first summarize_ticket failed");

        // Status-only change: updated_at moves, content_updated_at doesn't
        // (mirrors TicketService::update_ticket's field-selection logic —
        // see ticket::models::Ticket's doc comment).
        let ticket_repo = TicketRepository::new(&db);
        ticket_repo
            .update_ticket(
                group_id,
                ticket_id,
                doc! { "status": "closed", "updated_at": BsonDateTime::now() },
            )
            .await
            .expect("update_ticket failed");

        let response = service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("second summarize_ticket failed");

        assert!(response.cached);
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 1);
    });
}

// 5. Summary and analysis are cached independently: generating one doesn't
// mark the other as cached too, but each is still cached on its own.
#[test]
fn test_analyze_ticket_is_cached_independently_of_summary() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("a summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .summarize_ticket(member_id, group_id, ticket_id)
            .await
            .expect("summarize_ticket failed");
        let analysis = service
            .analyze_ticket(member_id, group_id, ticket_id)
            .await
            .expect("analyze_ticket failed");

        assert!(!analysis.cached);
        assert_eq!(analysis.severity_prediction, "high");
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider.analyze_calls.load(Ordering::SeqCst), 1);

        let second = service
            .analyze_ticket(member_id, group_id, ticket_id)
            .await
            .expect("second analyze_ticket failed");
        assert!(second.cached);
        assert_eq!(provider.analyze_calls.load(Ordering::SeqCst), 1);
    });
}

// 6. A non-member is rejected before the provider is ever called.
#[test]
fn test_summarize_ticket_rejects_non_member() {
    support::runtime().block_on(async {
        let (db, group_id, _member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());
        let outsider_id = ObjectId::new();

        let result = service
            .summarize_ticket(outsider_id, group_id, ticket_id)
            .await;

        assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
        assert_eq!(provider.summarize_calls.load(Ordering::SeqCst), 0);
    });
}

// 7. An unknown ticket_id 404s (rather than, say, generating a summary for
// nothing).
#[test]
fn test_summarize_ticket_404s_on_unknown_ticket() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, _ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider);

        let result = service
            .summarize_ticket(member_id, group_id, ObjectId::new())
            .await;

        assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
    });
}

// 8. First report call has nothing cached, so it computes real stats from
// the seeded tickets and calls the provider for the narrative.
#[test]
fn test_generate_group_report_computes_stats_and_calls_provider() {
    support::runtime().block_on(async {
        let (db, owner_id, _member_id, group_id) = setup_for_report().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        let report = service
            .generate_group_report(owner_id, group_id)
            .await
            .expect("generate_group_report failed");

        assert!(!report.cached);
        assert_eq!(report.data.total_tickets, 3);
        assert_eq!(report.data.open_tickets, 2);
        assert_eq!(report.data.closed_tickets, 1);
        assert_eq!(report.data.priority_breakdown.low, 1);
        assert_eq!(report.data.priority_breakdown.high, 1);
        assert_eq!(report.data.priority_breakdown.critical, 1);
        assert_eq!(report.data.narrative, "a narrative");
        assert_eq!(provider.narrate_calls.load(Ordering::SeqCst), 1);
    });
}

// 9. A second call within the TTL is served from cache; the provider isn't
// called again.
#[test]
fn test_generate_group_report_uses_cache_within_ttl() {
    support::runtime().block_on(async {
        let (db, owner_id, _member_id, group_id) = setup_for_report().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        let first = service
            .generate_group_report(owner_id, group_id)
            .await
            .expect("first generate_group_report failed");
        let second = service
            .generate_group_report(owner_id, group_id)
            .await
            .expect("second generate_group_report failed");

        assert!(second.cached);
        assert_eq!(second.data, first.data);
        assert_eq!(provider.narrate_calls.load(Ordering::SeqCst), 1);
    });
}

// 10. A Contributor (non-Group-Admin) is rejected before the provider is
// ever called.
#[test]
fn test_generate_group_report_requires_group_admin() {
    support::runtime().block_on(async {
        let (db, _owner_id, member_id, group_id) = setup_for_report().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        let result = service.generate_group_report(member_id, group_id).await;

        assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
        assert_eq!(provider.narrate_calls.load(Ordering::SeqCst), 0);
    });
}

// 11. Sending a message calls the provider and persists both the user's
// message and the assistant's reply, correctly attributed.
#[test]
fn test_send_chat_message_persists_both_messages() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        let response = service
            .send_chat_message(member_id, group_id, ticket_id, "Any workaround?".to_string())
            .await
            .expect("send_chat_message failed");

        assert_eq!(response.user_message.role, ChatRole::User);
        assert_eq!(response.user_message.content, "Any workaround?");
        assert_eq!(response.user_message.user_id, Some(member_id.to_hex()));
        assert_eq!(response.assistant_message.role, ChatRole::Assistant);
        assert_eq!(response.assistant_message.content, "a chat reply");
        assert_eq!(response.assistant_message.user_id, None);
        assert_eq!(provider.chat_calls.load(Ordering::SeqCst), 1);

        let stored = service
            .list_chat_messages(member_id, group_id, ticket_id)
            .await
            .expect("list_chat_messages failed");
        assert_eq!(stored.len(), 2);
    });
}

// 12. A second message includes the first exchange in the transcript sent to
// the provider — history isn't dropped between turns.
#[test]
fn test_send_chat_message_includes_prior_history_in_transcript() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .send_chat_message(member_id, group_id, ticket_id, "First question".to_string())
            .await
            .expect("first send_chat_message failed");
        // First call's transcript is empty — nothing happened yet.
        assert_eq!(*provider.last_chat_transcript.lock().unwrap(), "");

        service
            .send_chat_message(member_id, group_id, ticket_id, "Follow-up".to_string())
            .await
            .expect("second send_chat_message failed");

        let transcript = provider.last_chat_transcript.lock().unwrap().clone();
        assert!(transcript.contains("First question"));
        assert!(transcript.contains("a chat reply"));
    });
}

// 13. A non-member is rejected before the provider is ever called.
#[test]
fn test_send_chat_message_rejects_non_member() {
    support::runtime().block_on(async {
        let (db, group_id, _member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());
        let outsider_id = ObjectId::new();

        let result = service
            .send_chat_message(outsider_id, group_id, ticket_id, "hi".to_string())
            .await;

        assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
        assert_eq!(provider.chat_calls.load(Ordering::SeqCst), 0);
    });
}

// 14. An unknown ticket_id 404s.
#[test]
fn test_send_chat_message_404s_on_unknown_ticket() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, _ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider);

        let result = service
            .send_chat_message(member_id, group_id, ObjectId::new(), "hi".to_string())
            .await;

        assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
    });
}

// 15. The 11th message within an hour from the same user is rejected —
// confirmed with user: 10 messages/hour.
#[test]
fn test_send_chat_message_enforces_rate_limit() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        for i in 0..10 {
            service
                .send_chat_message(member_id, group_id, ticket_id, format!("message {i}"))
                .await
                .unwrap_or_else(|_| panic!("message {i} should be within the rate limit"));
        }
        assert_eq!(provider.chat_calls.load(Ordering::SeqCst), 10);

        let result = service
            .send_chat_message(member_id, group_id, ticket_id, "one too many".to_string())
            .await;

        assert!(matches!(result, Err(ApiError::RateLimited(_))), "{result:?}");
        // The rejected call never reached the provider.
        assert_eq!(provider.chat_calls.load(Ordering::SeqCst), 10);
    });
}

// 16. The rate limit is scoped per user, not per ticket: a different member
// of the same group has their own budget.
#[test]
fn test_send_chat_message_rate_limit_is_scoped_per_user() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());
        let other_member_id = ObjectId::new();
        let group_repo = GroupRepository::new(&db);
        group_repo
            .insert_member(group_id, other_member_id, Role::Contributor)
            .await
            .expect("insert_member failed");

        for i in 0..10 {
            service
                .send_chat_message(member_id, group_id, ticket_id, format!("message {i}"))
                .await
                .unwrap_or_else(|_| panic!("message {i} should be within the rate limit"));
        }

        let result = service
            .send_chat_message(other_member_id, group_id, ticket_id, "hi".to_string())
            .await;

        assert!(result.is_ok(), "{result:?}");
    });
}

// 17. list_chat_messages returns the full thread oldest-first.
#[test]
fn test_list_chat_messages_returns_oldest_first() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .send_chat_message(member_id, group_id, ticket_id, "first".to_string())
            .await
            .expect("first send_chat_message failed");
        service
            .send_chat_message(member_id, group_id, ticket_id, "second".to_string())
            .await
            .expect("second send_chat_message failed");

        let messages = service
            .list_chat_messages(member_id, group_id, ticket_id)
            .await
            .expect("list_chat_messages failed");

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].content, "first");
        assert_eq!(messages[2].content, "second");
    });
}

// 18. "New chat" clears every message on the ticket's thread.
#[test]
fn test_clear_chat_removes_all_messages() {
    support::runtime().block_on(async {
        let (db, group_id, member_id, ticket_id) = setup().await;
        let provider = FakeProvider::new("summary", default_analysis());
        let service = AiService::with_provider(&db, provider.clone());

        service
            .send_chat_message(member_id, group_id, ticket_id, "hi".to_string())
            .await
            .expect("send_chat_message failed");

        service
            .clear_chat(member_id, group_id, ticket_id)
            .await
            .expect("clear_chat failed");

        let messages = service
            .list_chat_messages(member_id, group_id, ticket_id)
            .await
            .expect("list_chat_messages failed");
        assert!(messages.is_empty());
    });
}
