use mongodb::bson::{DateTime as BsonDateTime, doc, oid::ObjectId};
use resolve::ai::repository::AiRepository;

mod support;

async fn setup() -> AiRepository {
    let db = support::shared_client().await.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state (same
    // pattern as group_repository_tests::setup).
    db.collection::<mongodb::bson::Document>("ai_ticket_insights")
        .drop()
        .await
        .expect("failed to drop ai_ticket_insights collection");
    db.collection::<mongodb::bson::Document>("ai_group_reports")
        .drop()
        .await
        .expect("failed to drop ai_group_reports collection");

    AiRepository::new(&db)
}

fn oid() -> ObjectId {
    ObjectId::new()
}

// 1. find_insight returns None when nothing has ever been generated.
#[test]
fn test_find_insight_missing_returns_none() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let found = repo
            .find_insight(oid(), oid())
            .await
            .expect("find_insight failed");
        assert!(found.is_none());
    });
}

// 2. upsert_summary creates a new document with the summary fields set and
// the analysis fields still empty.
#[test]
fn test_upsert_summary_creates_document() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let ts = BsonDateTime::now();

        let insight = repo
            .upsert_summary(group_id, ticket_id, "a concise summary", ts)
            .await
            .expect("upsert_summary failed");

        assert!(insight.id.is_some());
        assert_eq!(insight.group_id, group_id);
        assert_eq!(insight.ticket_id, ticket_id);
        assert_eq!(insight.summary.as_deref(), Some("a concise summary"));
        assert_eq!(insight.summary_source_updated_at, Some(ts));
        assert!(insight.severity_prediction.is_none());
        assert!(insight.is_summary_fresh(ts));
        assert!(!insight.is_analysis_fresh(ts));
    });
}

// 3. upsert_analysis on top of an existing summary leaves the summary intact
// — the two field groups are independently upserted on the same document.
#[test]
fn test_upsert_analysis_preserves_existing_summary() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let ts = BsonDateTime::now();

        repo.upsert_summary(group_id, ticket_id, "summary text", ts)
            .await
            .expect("upsert_summary failed");
        let insight = repo
            .upsert_analysis(group_id, ticket_id, "high", "restart the service", "bug", ts)
            .await
            .expect("upsert_analysis failed");

        assert_eq!(insight.summary.as_deref(), Some("summary text"));
        assert_eq!(insight.severity_prediction.as_deref(), Some("high"));
        assert_eq!(insight.suggested_fix.as_deref(), Some("restart the service"));
        assert_eq!(insight.classification.as_deref(), Some("bug"));
        assert!(insight.is_summary_fresh(ts));
        assert!(insight.is_analysis_fresh(ts));
    });
}

// 4. A later upsert_summary call (simulating a ticket edit + re-summarize)
// overwrites the summary and its timestamp, without touching analysis.
#[test]
fn test_upsert_summary_refreshes_existing_document() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let ts1 = BsonDateTime::from_millis(1_000);
        let ts2 = BsonDateTime::from_millis(2_000);

        repo.upsert_summary(group_id, ticket_id, "old summary", ts1)
            .await
            .expect("first upsert_summary failed");
        repo.upsert_analysis(group_id, ticket_id, "low", "no action", "feature", ts1)
            .await
            .expect("upsert_analysis failed");

        let refreshed = repo
            .upsert_summary(group_id, ticket_id, "new summary", ts2)
            .await
            .expect("second upsert_summary failed");

        assert_eq!(refreshed.summary.as_deref(), Some("new summary"));
        assert!(refreshed.is_summary_fresh(ts2));
        // Analysis was generated for ts1 and the ticket has "moved on" to
        // ts2, so it now reads as stale — correct, since only the summary
        // was actually regenerated here.
        assert!(!refreshed.is_analysis_fresh(ts2));
        assert_eq!(refreshed.severity_prediction.as_deref(), Some("low"));
    });
}

// 5. Insights are isolated per (group_id, ticket_id) pair — a lookup with
// either id from a different pair finds nothing, same multi-tenancy
// guarantee as TicketRepository::find_by_id.
#[test]
fn test_find_insight_is_scoped_to_group_and_ticket() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let ts = BsonDateTime::now();

        repo.upsert_summary(group_id, ticket_id, "summary", ts)
            .await
            .expect("upsert_summary failed");

        assert!(
            repo.find_insight(oid(), ticket_id)
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            repo.find_insight(group_id, oid())
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            repo.find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_some()
        );
    });
}

// 6. delete_by_ticket removes only the matching ticket's insight.
#[test]
fn test_delete_by_ticket_removes_only_that_ticket() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let other_ticket_id = oid();
        let ts = BsonDateTime::now();

        repo.upsert_summary(group_id, ticket_id, "summary", ts)
            .await
            .expect("upsert_summary failed");
        repo.upsert_summary(group_id, other_ticket_id, "other summary", ts)
            .await
            .expect("upsert_summary failed");

        let deleted = repo
            .delete_by_ticket(group_id, ticket_id)
            .await
            .expect("delete_by_ticket failed");
        assert_eq!(deleted, 1);

        assert!(
            repo.find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            repo.find_insight(group_id, other_ticket_id)
                .await
                .expect("find_insight failed")
                .is_some()
        );
    });
}

// 7. insert_report + find_latest_report round-trip, and "latest" really means
// most recently generated when multiple reports exist for the same group.
#[test]
fn test_find_latest_report_returns_most_recent() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let generated_by = oid();

        assert!(
            repo.find_latest_report(group_id)
                .await
                .expect("find_latest_report failed")
                .is_none()
        );

        repo.insert_report(group_id, doc! { "open_tickets": 3 }, generated_by)
            .await
            .expect("insert_report failed");
        // Mongo's Date type has millisecond resolution, so a real gap is
        // needed for the two reports to sort deterministically.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let second = repo
            .insert_report(group_id, doc! { "open_tickets": 5 }, generated_by)
            .await
            .expect("insert_report failed");

        let latest = repo
            .find_latest_report(group_id)
            .await
            .expect("find_latest_report failed")
            .expect("expected a report");
        assert_eq!(latest.id, second.id);
        assert_eq!(latest.report_data, doc! { "open_tickets": 5 });
    });
}

// 8. delete_by_group clears both insights and reports for the group, and
// nothing else.
#[test]
fn test_delete_by_group_clears_insights_and_reports() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let other_group_id = oid();
        let ticket_id = oid();
        let ts = BsonDateTime::now();

        repo.upsert_summary(group_id, ticket_id, "summary", ts)
            .await
            .expect("upsert_summary failed");
        repo.insert_report(group_id, doc! { "open_tickets": 1 }, oid())
            .await
            .expect("insert_report failed");
        repo.upsert_summary(other_group_id, oid(), "other group summary", ts)
            .await
            .expect("upsert_summary failed");

        repo.delete_by_group(group_id)
            .await
            .expect("delete_by_group failed");

        assert!(
            repo.find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            repo.find_latest_report(group_id)
                .await
                .expect("find_latest_report failed")
                .is_none()
        );
        assert!(
            repo.find_insight(other_group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_none() // different ticket_id was used for the other group
        );
    });
}
