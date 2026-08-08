use mongodb::{IndexModel, bson::Document, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::activity::models::ActivityEventType;
use resolve::activity::service::ActivityService;
use resolve::comment::service::CommentService;
use resolve::errors::ApiError;
use resolve::ticket::models::{
    CreateTicketInput, TicketPriority, TicketStatus, UpdateTicketRequest,
};
use resolve::ticket::repository::TicketRepository;
use resolve::ticket::service::TicketService;

use resolve::group::service::GroupService;

mod support;

async fn setup() -> (GroupService, TicketService, CommentService, ActivityService) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "groups",
        "group_members",
        "tickets",
        "counters",
        "comments",
        "ticket_activity",
    ] {
        db.collection::<Document>(collection)
            .drop()
            .await
            .unwrap_or_else(|_| panic!("failed to drop {collection} collection"));
    }

    db.collection::<Document>("group_members")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create group_members compound index");

    (
        GroupService::new(&db),
        TicketService::new(&db),
        CommentService::new(&db),
        ActivityService::new(&db),
    )
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
}

fn assert_not_found<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
}

// Allocates a ticket_number the same way TicketService does, so the counter
// document exists too (matches comment_api_tests.rs's seed_ticket) — but goes
// through TicketService::create_ticket here (not the repository directly),
// since this file is asserting on the activity events *that call* produces.
async fn seed_ticket(
    tickets: &TicketService,
    group_id: ObjectId,
    created_by: ObjectId,
) -> ObjectId {
    let ticket = tickets
        .create_ticket(
            created_by,
            group_id,
            resolve::ticket::models::CreateTicketRequest {
                title: "a ticket".to_string(),
                description: "description".to_string(),
                priority: TicketPriority::Low,
            },
        )
        .await
        .expect("create ticket failed");
    ObjectId::parse_str(&ticket.id).unwrap()
}

// A second ticket that bypasses TicketService::create_ticket entirely, for
// tests that need a ticket with no activity history of its own yet.
async fn seed_bare_ticket(group_id: ObjectId, created_by: ObjectId) -> ObjectId {
    let db = support::shared_client().await.database("resolve_test");
    let repo = TicketRepository::new(&db);
    let ticket_number = repo
        .next_ticket_number(group_id)
        .await
        .expect("counter allocation failed");
    let ticket = repo
        .insert_ticket(CreateTicketInput {
            group_id,
            ticket_number,
            title: "bare ticket".to_string(),
            description: "description".to_string(),
            priority: TicketPriority::Low,
            created_by,
        })
        .await
        .expect("ticket insert failed");
    ticket.id.expect("insert_ticket always returns an id")
}

// 1. Creating a ticket records a single ticket_created event.
#[test]
fn test_create_ticket_records_ticket_created_event() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event_type, ActivityEventType::TicketCreated);
        assert!(entries[0].old_value.is_none());
        assert!(entries[0].new_value.is_none());
        assert_eq!(entries[0].actor_id, owner.to_hex());
    });
}

// 2. Changing status records a status_changed event with old/new values.
#[test]
fn test_update_ticket_status_records_status_changed() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        tickets
            .update_ticket(
                owner,
                group_id,
                ticket_id,
                UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: None,
                    status: Some(TicketStatus::Closed),
                },
            )
            .await
            .expect("update failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        let status_event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::StatusChanged)
            .expect("status_changed event missing");
        assert_eq!(status_event.old_value.as_deref(), Some("open"));
        assert_eq!(status_event.new_value.as_deref(), Some("closed"));
    });
}

// 3. Changing priority records a priority_changed event with old/new values.
#[test]
fn test_update_ticket_priority_records_priority_changed() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        tickets
            .update_ticket(
                owner,
                group_id,
                ticket_id,
                UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: Some(TicketPriority::Critical),
                    status: None,
                },
            )
            .await
            .expect("update failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        let priority_event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::PriorityChanged)
            .expect("priority_changed event missing");
        assert_eq!(priority_event.old_value.as_deref(), Some("low"));
        assert_eq!(priority_event.new_value.as_deref(), Some("critical"));
    });
}

// 4. Changing the title records old/new text; changing the description
// records the event with no text (see ActivityEventType::DescriptionChanged).
#[test]
fn test_update_ticket_title_and_description_record_distinct_events() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        tickets
            .update_ticket(
                owner,
                group_id,
                ticket_id,
                UpdateTicketRequest {
                    title: Some("new title".to_string()),
                    description: Some("new description".to_string()),
                    priority: None,
                    status: None,
                },
            )
            .await
            .expect("update failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");

        let title_event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::TitleChanged)
            .expect("title_changed event missing");
        assert_eq!(title_event.old_value.as_deref(), Some("a ticket"));
        assert_eq!(title_event.new_value.as_deref(), Some("new title"));

        let description_event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::DescriptionChanged)
            .expect("description_changed event missing");
        assert!(description_event.old_value.is_none());
        assert!(description_event.new_value.is_none());
    });
}

// 5. Setting a field to the value it already has does not record an event —
// only genuine changes are logged.
#[test]
fn test_update_ticket_unchanged_value_records_no_event() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        let before = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        assert_eq!(before.len(), 1, "only the ticket_created event so far");

        tickets
            .update_ticket(
                owner,
                group_id,
                ticket_id,
                UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: Some(TicketPriority::Low),
                    status: None,
                },
            )
            .await
            .expect("update failed");

        let after = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        assert_eq!(
            after.len(),
            1,
            "priority was re-set to its existing value, so no priority_changed event should appear"
        );
    });
}

// 6. Creating and deleting a comment each record their own event.
#[test]
fn test_comment_add_and_delete_record_events() {
    support::runtime().block_on(async {
        let (groups, tickets, comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        let comment = comments
            .create_comment(owner, group_id, ticket_id, "hello".to_string(), None)
            .await
            .expect("create comment failed");
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        comments
            .delete_comment(owner, group_id, ticket_id, comment_id)
            .await
            .expect("delete comment failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");

        let added = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::CommentAdded)
            .expect("comment_added event missing");
        assert_eq!(added.comment_id.as_deref(), Some(comment.id.as_str()));

        let deleted = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::CommentDeleted)
            .expect("comment_deleted event missing");
        assert_eq!(deleted.comment_id.as_deref(), Some(comment.id.as_str()));
    });
}

// 7. Entries come back newest-first.
#[test]
fn test_list_activity_returns_newest_first() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        tickets
            .update_ticket(
                owner,
                group_id,
                ticket_id,
                UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: None,
                    status: Some(TicketStatus::Closed),
                },
            )
            .await
            .expect("update failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].event_type, ActivityEventType::StatusChanged);
        assert_eq!(entries[1].event_type, ActivityEventType::TicketCreated);
    });
}

// 8. A non-member cannot read a ticket's activity.
#[test]
fn test_list_activity_non_member_forbidden() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        let result = activity.list_activity(oid(), group_id, ticket_id).await;
        assert_forbidden(result);
    });
}

// 9. Same cross-tenant guard as comments: a ticket_id from another group 404s
// rather than leaking that group's activity.
#[test]
fn test_list_activity_with_ticket_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, activity) = setup().await;
        let owner_x = oid();
        let owner_y = oid();

        let group_x = groups
            .create_group(owner_x, "X".to_string())
            .await
            .expect("create group X failed");
        let group_x_id = ObjectId::parse_str(&group_x.id).unwrap();

        let group_y = groups
            .create_group(owner_y, "Y".to_string())
            .await
            .expect("create group Y failed");
        let group_y_id = ObjectId::parse_str(&group_y.id).unwrap();

        let ticket_in_y = seed_ticket(&tickets, group_y_id, owner_y).await;

        let result = activity
            .list_activity(owner_x, group_x_id, ticket_in_y)
            .await;
        assert_not_found(result);
    });
}

// 10. Deleting a ticket cascades its activity log.
#[test]
fn test_delete_ticket_cascades_activity() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(&tickets, group_id, owner).await;

        tickets
            .delete_ticket(owner, group_id, ticket_id)
            .await
            .expect("delete ticket failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_activity")
            .count_documents(doc! { "ticket_id": ticket_id })
            .await
            .expect("count failed");
        assert_eq!(
            count, 0,
            "activity entries were orphaned by ticket deletion"
        );
    });
}

// 11. Deleting a whole group cascades activity for every ticket in it.
#[test]
fn test_delete_group_cascades_activity() {
    support::runtime().block_on(async {
        let (groups, tickets, _comments, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let _ticket_id = seed_ticket(&tickets, group_id, owner).await;
        let _bare_ticket_id = seed_bare_ticket(group_id, owner).await;

        groups
            .delete_group(owner, group_id)
            .await
            .expect("delete group failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_activity")
            .count_documents(doc! { "group_id": group_id })
            .await
            .expect("count failed");
        assert_eq!(count, 0, "activity entries were orphaned by group deletion");
    });
}
