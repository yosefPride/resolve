use mongodb::bson::{DateTime as BsonDateTime, doc, oid::ObjectId};
use resolve::activity::repository::ActivityRepository;
use resolve::ai::models::ChatRole;
use resolve::ai::repository::AiRepository;
use resolve::comment::repository::CommentRepository;
use resolve::group::models::{CreateGroupInput, Role};
use resolve::group::repository::GroupRepository;
use resolve::group::service::purge_group_data;
use resolve::ticket::models::{CreateTicketInput, TicketPriority};
use resolve::ticket::repository::TicketRepository;
use resolve::ticket::service::TicketService;

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
    db.collection::<mongodb::bson::Document>("ai_chat_messages")
        .drop()
        .await
        .expect("failed to drop ai_chat_messages collection");
    db.collection::<mongodb::bson::Document>("ai_conversations")
        .drop()
        .await
        .expect("failed to drop ai_conversations collection");

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
            .upsert_analysis(
                group_id,
                ticket_id,
                "high",
                "restart the service",
                "bug",
                ts,
            )
            .await
            .expect("upsert_analysis failed");

        assert_eq!(insight.summary.as_deref(), Some("summary text"));
        assert_eq!(insight.severity_prediction.as_deref(), Some("high"));
        assert_eq!(
            insight.suggested_fix.as_deref(),
            Some("restart the service")
        );
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

// 9. create_conversation + list_conversations + find_conversation round-trip.
#[test]
fn test_create_list_find_conversation_round_trip() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();

        let created = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed");
        assert!(created.id.is_some());
        assert_eq!(created.title, None);

        let listed = repo
            .list_conversations(group_id, ticket_id, user_id)
            .await
            .expect("list_conversations failed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let found = repo
            .find_conversation(created.id.unwrap())
            .await
            .expect("find_conversation failed")
            .expect("conversation should exist");
        assert_eq!(found.user_id, user_id);
    });
}

// 10. list_conversations only returns conversations owned by that user, even
// when other users have conversations on the same ticket.
#[test]
fn test_list_conversations_is_scoped_to_user() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let other_user_id = oid();

        repo.create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed");
        repo.create_conversation(group_id, ticket_id, other_user_id)
            .await
            .expect("create_conversation failed");

        let listed = repo
            .list_conversations(group_id, ticket_id, user_id)
            .await
            .expect("list_conversations failed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].user_id, user_id);
    });
}

// 11. list_conversations sorts most-recently-active first — touch_conversation
// bumps updated_at on the older one, which should then sort ahead.
#[test]
fn test_list_conversations_orders_most_recently_active_first() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();

        let older = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let newer = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed");

        // Touch the older one after the newer one was created, so it should
        // now sort first.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        repo.touch_conversation(older.id.unwrap(), "a title")
            .await
            .expect("touch_conversation failed");

        let listed = repo
            .list_conversations(group_id, ticket_id, user_id)
            .await
            .expect("list_conversations failed");
        assert_eq!(listed[0].id, older.id);
        assert_eq!(listed[1].id, newer.id);
    });
}

// 12. touch_conversation sets the title only the first time — a second call
// with a different title leaves the first one in place — and bumps
// updated_at every time.
#[test]
fn test_touch_conversation_sets_title_once_and_always_bumps_updated_at() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let created = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed");
        let conversation_id = created.id.unwrap();

        repo.touch_conversation(conversation_id, "first title")
            .await
            .expect("first touch_conversation failed");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        repo.touch_conversation(conversation_id, "second title")
            .await
            .expect("second touch_conversation failed");

        let found = repo
            .find_conversation(conversation_id)
            .await
            .expect("find_conversation failed")
            .expect("conversation should exist");
        assert_eq!(found.title, Some("first title".to_string()));
        assert!(found.updated_at > created.updated_at);
    });
}

// 13. insert_chat_message + list_conversation_messages round-trip,
// oldest-first.
#[test]
fn test_insert_and_list_conversation_messages_oldest_first() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let conversation_id = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();

        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(user_id),
            "hi",
        )
        .await
        .expect("insert_chat_message (user) failed");
        // Mongo's Date type has millisecond resolution — a real gap is
        // needed for the two messages to sort deterministically.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::Assistant,
            None,
            "hello",
        )
        .await
        .expect("insert_chat_message (assistant) failed");

        let messages = repo
            .list_conversation_messages(conversation_id)
            .await
            .expect("list_conversation_messages failed");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, ChatRole::User);
        assert_eq!(messages[0].content, "hi");
        assert_eq!(messages[0].user_id, Some(user_id));
        assert_eq!(messages[1].role, ChatRole::Assistant);
        assert_eq!(messages[1].content, "hello");
        assert_eq!(messages[1].user_id, None);
    });
}

// 14. Messages are isolated per conversation_id, same idea as the old
// group_id/ticket_id scoping guarantee.
#[test]
fn test_list_conversation_messages_is_scoped_to_conversation() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let conversation_id = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();

        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(oid()),
            "hi",
        )
        .await
        .expect("insert_chat_message failed");

        assert!(
            repo.list_conversation_messages(oid())
                .await
                .expect("list_conversation_messages failed")
                .is_empty()
        );
    });
}

// 15. delete_conversation removes only the matching conversation and its own
// messages, leaving a different conversation (even on the same ticket)
// untouched.
#[test]
fn test_delete_conversation_removes_only_that_conversation() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let to_delete = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();
        let to_keep = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();

        repo.insert_chat_message(
            to_delete,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(oid()),
            "hi",
        )
        .await
        .expect("insert_chat_message failed");
        repo.insert_chat_message(
            to_keep,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(oid()),
            "hi",
        )
        .await
        .expect("insert_chat_message failed");

        repo.delete_conversation(to_delete)
            .await
            .expect("delete_conversation failed");

        assert!(
            repo.find_conversation(to_delete)
                .await
                .expect("find_conversation failed")
                .is_none()
        );
        assert!(
            repo.list_conversation_messages(to_delete)
                .await
                .expect("list_conversation_messages failed")
                .is_empty()
        );
        assert!(repo.find_conversation(to_keep).await.unwrap().is_some());
        assert_eq!(
            repo.list_conversation_messages(to_keep)
                .await
                .expect("list_conversation_messages failed")
                .len(),
            1
        );
    });
}

// 16. count_recent_user_messages only counts role: user messages by that
// user within the window — an assistant message and a different user's
// message are both excluded, and a message before `since` doesn't count.
// Global per user, not scoped to a conversation_id.
#[test]
fn test_count_recent_user_messages_filters_role_user_and_window() {
    support::runtime().block_on(async {
        let repo = setup().await;
        let group_id = oid();
        let ticket_id = oid();
        let user_id = oid();
        let other_user_id = oid();
        let conversation_id = repo
            .create_conversation(group_id, ticket_id, user_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();

        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(user_id),
            "1",
        )
        .await
        .expect("insert_chat_message failed");
        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::Assistant,
            None,
            "reply",
        )
        .await
        .expect("insert_chat_message failed");
        repo.insert_chat_message(
            conversation_id,
            group_id,
            ticket_id,
            ChatRole::User,
            Some(other_user_id),
            "1",
        )
        .await
        .expect("insert_chat_message failed");

        let since_start_of_test = BsonDateTime::from_millis(0);
        let count = repo
            .count_recent_user_messages(user_id, since_start_of_test)
            .await
            .expect("count_recent_user_messages failed");
        assert_eq!(count, 1);

        let since_future =
            BsonDateTime::from_millis(BsonDateTime::now().timestamp_millis() + 60_000);
        let count_after_window = repo
            .count_recent_user_messages(user_id, since_future)
            .await
            .expect("count_recent_user_messages failed");
        assert_eq!(count_after_window, 0);
    });
}

// The tests below prove the cascade wiring itself (TicketService::
// delete_ticket, purge_group_data) actually calls through to AiRepository —
// distinct from the delete_by_ticket/delete_by_group tests above, which only
// prove the repository methods work in isolation. Neither ticket_api_tests.rs
// nor group_api_tests.rs/admin_api_tests.rs ever touch the ai_* collections,
// so this is the only place a regression here (e.g. someone removing the
// ai_repo.delete_by_ticket call from TicketService::delete_ticket) would be
// caught.

// 17. Deleting a ticket removes its AI insight, chat messages, and
// conversations.
#[test]
fn test_ticket_delete_cascades_to_ai_insight() {
    support::runtime().block_on(async {
        let db = support::shared_client().await.database("resolve_test");
        for collection in [
            "ai_ticket_insights",
            "ai_group_reports",
            "ai_chat_messages",
            "ai_conversations",
            "groups",
            "group_members",
            "tickets",
            "counters",
        ] {
            db.collection::<mongodb::bson::Document>(collection)
                .drop()
                .await
                .unwrap_or_else(|_| panic!("failed to drop {collection} collection"));
        }

        let ai_repo = AiRepository::new(&db);
        let group_repo = GroupRepository::new(&db);
        let ticket_repo = TicketRepository::new(&db);
        let ticket_service = TicketService::new(&db);

        let owner_id = oid();
        let group = group_repo
            .create_group(CreateGroupInput {
                name: "Cascade Test Group".to_string(),
                owner_id,
            })
            .await
            .expect("create_group failed");
        let group_id = group.id.expect("created group has an id");
        // GroupRepository::create_group only creates the group document —
        // the creator's own membership row is a separate insert_member call
        // (normally GroupService::create_group's job).
        group_repo
            .insert_member(group_id, owner_id, Role::GroupAdmin)
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
                title: "a ticket".to_string(),
                description: "a description".to_string(),
                priority: TicketPriority::Low,
                created_by: owner_id,
            })
            .await
            .expect("insert_ticket failed");
        let ticket_id = ticket.id.expect("created ticket has an id");

        ai_repo
            .upsert_summary(group_id, ticket_id, "a summary", BsonDateTime::now())
            .await
            .expect("upsert_summary failed");
        let conversation_id = ai_repo
            .create_conversation(group_id, ticket_id, owner_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();
        ai_repo
            .insert_chat_message(
                conversation_id,
                group_id,
                ticket_id,
                ChatRole::User,
                Some(owner_id),
                "hi",
            )
            .await
            .expect("insert_chat_message failed");
        assert!(
            ai_repo
                .find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_some()
        );

        ticket_service
            .delete_ticket(owner_id, group_id, ticket_id)
            .await
            .expect("delete_ticket failed");

        assert!(
            ai_repo
                .find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            ai_repo
                .list_conversation_messages(conversation_id)
                .await
                .expect("list_conversation_messages failed")
                .is_empty()
        );
        assert!(
            ai_repo
                .find_conversation(conversation_id)
                .await
                .expect("find_conversation failed")
                .is_none()
        );
    });
}

// 18. Deleting a group removes its AI insights, reports, chat messages, and
// conversations.
#[test]
fn test_group_delete_cascades_to_ai_data() {
    support::runtime().block_on(async {
        let db = support::shared_client().await.database("resolve_test");
        for collection in [
            "ai_ticket_insights",
            "ai_group_reports",
            "ai_chat_messages",
            "ai_conversations",
            "groups",
            "group_members",
            "tickets",
            "counters",
        ] {
            db.collection::<mongodb::bson::Document>(collection)
                .drop()
                .await
                .unwrap_or_else(|_| panic!("failed to drop {collection} collection"));
        }

        let ai_repo = AiRepository::new(&db);
        let group_repo = GroupRepository::new(&db);
        let ticket_repo = TicketRepository::new(&db);
        let comment_repo = CommentRepository::new(&db);
        let activity_repo = ActivityRepository::new(&db);

        let owner_id = oid();
        let group = group_repo
            .create_group(CreateGroupInput {
                name: "Cascade Test Group 2".to_string(),
                owner_id,
            })
            .await
            .expect("create_group failed");
        let group_id = group.id.expect("created group has an id");

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
                priority: TicketPriority::Low,
                created_by: owner_id,
            })
            .await
            .expect("insert_ticket failed");
        let ticket_id = ticket.id.expect("created ticket has an id");

        ai_repo
            .upsert_summary(group_id, ticket_id, "a summary", BsonDateTime::now())
            .await
            .expect("upsert_summary failed");
        ai_repo
            .insert_report(group_id, doc! { "total_tickets": 1 }, owner_id)
            .await
            .expect("insert_report failed");
        let conversation_id = ai_repo
            .create_conversation(group_id, ticket_id, owner_id)
            .await
            .expect("create_conversation failed")
            .id
            .unwrap();
        ai_repo
            .insert_chat_message(
                conversation_id,
                group_id,
                ticket_id,
                ChatRole::User,
                Some(owner_id),
                "hi",
            )
            .await
            .expect("insert_chat_message failed");

        purge_group_data(
            &group_repo,
            &ticket_repo,
            &comment_repo,
            &ai_repo,
            &activity_repo,
            group_id,
        )
        .await
        .expect("purge_group_data failed");

        assert!(
            ai_repo
                .find_insight(group_id, ticket_id)
                .await
                .expect("find_insight failed")
                .is_none()
        );
        assert!(
            ai_repo
                .list_conversation_messages(conversation_id)
                .await
                .expect("list_conversation_messages failed")
                .is_empty()
        );
        assert!(
            ai_repo
                .find_conversation(conversation_id)
                .await
                .expect("find_conversation failed")
                .is_none()
        );
        assert!(
            ai_repo
                .find_latest_report(group_id)
                .await
                .expect("find_latest_report failed")
                .is_none()
        );
    });
}
