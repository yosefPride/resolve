use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mongodb::bson::{DateTime as BsonDateTime, Document, doc, oid::ObjectId};
use resolve::ai::client::{AiProvider, AnalysisResult};
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
    summary: Arc<Mutex<String>>,
    analysis: Arc<Mutex<AnalysisResult>>,
}

impl FakeProvider {
    fn new(summary: &str, analysis: AnalysisResult) -> Self {
        Self {
            summarize_calls: Arc::new(AtomicUsize::new(0)),
            analyze_calls: Arc::new(AtomicUsize::new(0)),
            summary: Arc::new(Mutex::new(summary.to_string())),
            analysis: Arc::new(Mutex::new(analysis)),
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
        // the edit's updated_at to collide with the cached fingerprint —
        // same reasoning as ai_repository_tests's report-ordering test. A
        // real Gemini round-trip takes far longer than 5ms, so this can't
        // happen in production.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // TicketRepository::update_ticket doesn't bump updated_at itself —
        // that's TicketService::update_ticket's job, which inserts it into
        // the changes doc before calling the repo. Going through the repo
        // directly here (skipping RBAC and other ticket-service concerns
        // this test doesn't care about) means updated_at must be set
        // explicitly, the same way the service does it.
        let ticket_repo = TicketRepository::new(&db);
        ticket_repo
            .update_ticket(
                group_id,
                ticket_id,
                doc! {
                    "description": "Now also broken on desktop Chrome.",
                    "updated_at": BsonDateTime::now(),
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

// 4. Summary and analysis are cached independently: generating one doesn't
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

// 5. A non-member is rejected before the provider is ever called.
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

// 6. An unknown ticket_id 404s (rather than, say, generating a summary for
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
