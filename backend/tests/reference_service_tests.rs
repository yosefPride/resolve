use mongodb::{IndexModel, bson::Document, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::activity::models::{ActivityEventType, LinkKind};
use resolve::activity::service::ActivityService;
use resolve::errors::ApiError;
use resolve::group::models::Role;
use resolve::group::service::GroupService;
use resolve::reference::models::CreateReferenceRequest;
use resolve::reference::service::ReferenceService;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse};
use resolve::ticket::service::TicketService;

mod support;

async fn setup() -> (GroupService, TicketService, ReferenceService, ActivityService) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "groups",
        "group_members",
        "tickets",
        "counters",
        "ticket_references",
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
        ReferenceService::new(&db),
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

async fn seed_ticket(tickets: &TicketService, group_id: ObjectId, created_by: ObjectId) -> TicketResponse {
    tickets
        .create_ticket(
            created_by,
            group_id,
            CreateTicketRequest {
                title: "a ticket".to_string(),
                description: "description".to_string(),
                priority: TicketPriority::Low,
            },
        )
        .await
        .expect("create ticket failed")
}

fn reference_request(url: &str, label: Option<&str>) -> CreateReferenceRequest {
    CreateReferenceRequest {
        label: label.map(str::to_string),
        url: url.to_string(),
    }
}

// 1. Any group member can attach a reference with an explicit label.
#[test]
fn test_create_reference_member_succeeds() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let reference = references
            .create_reference(
                owner,
                group_id,
                ticket_id,
                reference_request("https://example.com/doc", Some("Design doc")),
            )
            .await
            .expect("create reference failed");

        assert_eq!(reference.label, "Design doc");
        assert_eq!(reference.url, "https://example.com/doc");
        assert_eq!(reference.created_by, owner.to_hex());
    });
}

// 2. A blank/absent label falls back to the URL's host.
#[test]
fn test_create_reference_blank_label_derives_from_url_host() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let reference = references
            .create_reference(
                owner,
                group_id,
                ticket_id,
                reference_request("https://github.com/org/repo/pull/12", None),
            )
            .await
            .expect("create reference failed");

        assert_eq!(reference.label, "github.com");
    });
}

// 3. References list oldest first.
#[test]
fn test_list_references_oldest_first() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        references
            .create_reference(owner, group_id, ticket_id, reference_request("https://a.com", None))
            .await
            .expect("create first failed");
        references
            .create_reference(owner, group_id, ticket_id, reference_request("https://b.com", None))
            .await
            .expect("create second failed");

        let list = references
            .list_references(owner, group_id, ticket_id)
            .await
            .expect("list failed");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label, "a.com");
        assert_eq!(list[1].label, "b.com");
    });
}

// 4. A Group Admin (not the reference's creator) can still delete it.
#[test]
fn test_delete_reference_by_group_admin_non_owner_succeeds() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let contributor = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(owner, group_id, contributor, Role::Contributor)
            .await
            .expect("add member failed");
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let reference = references
            .create_reference(contributor, group_id, ticket_id, reference_request("https://a.com", None))
            .await
            .expect("create reference failed");
        let reference_id = ObjectId::parse_str(&reference.id).unwrap();

        references
            .delete_reference(owner, group_id, ticket_id, reference_id)
            .await
            .expect("group admin delete failed");
    });
}

// 5. A Contributor who neither created the reference nor is a Group Admin is
// forbidden from deleting it.
#[test]
fn test_delete_reference_by_other_contributor_forbidden() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let author = oid();
        let other = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(owner, group_id, author, Role::Contributor)
            .await
            .expect("add author failed");
        groups
            .add_member(owner, group_id, other, Role::Contributor)
            .await
            .expect("add other failed");
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let reference = references
            .create_reference(author, group_id, ticket_id, reference_request("https://a.com", None))
            .await
            .expect("create reference failed");
        let reference_id = ObjectId::parse_str(&reference.id).unwrap();

        let result = references.delete_reference(other, group_id, ticket_id, reference_id).await;
        assert_forbidden(result);
    });
}

// 6. Creating a reference logs a link_added event tagged link_kind=reference.
#[test]
fn test_create_reference_records_activity() {
    support::runtime().block_on(async {
        let (groups, tickets, references, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        references
            .create_reference(
                owner,
                group_id,
                ticket_id,
                reference_request("https://example.com", Some("Example")),
            )
            .await
            .expect("create reference failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        let event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkAdded)
            .expect("link_added missing");
        assert_eq!(event.link_kind, Some(LinkKind::Reference));
        assert_eq!(event.new_value.as_deref(), Some("Example"));
    });
}

// 7. Deleting a reference logs a link_removed event, also tagged reference.
#[test]
fn test_delete_reference_records_activity() {
    support::runtime().block_on(async {
        let (groups, tickets, references, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let reference = references
            .create_reference(
                owner,
                group_id,
                ticket_id,
                reference_request("https://example.com", Some("Example")),
            )
            .await
            .expect("create reference failed");
        let reference_id = ObjectId::parse_str(&reference.id).unwrap();

        references
            .delete_reference(owner, group_id, ticket_id, reference_id)
            .await
            .expect("delete reference failed");

        let entries = activity
            .list_activity(owner, group_id, ticket_id)
            .await
            .expect("list activity failed");
        let event = entries
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkRemoved)
            .expect("link_removed missing");
        assert_eq!(event.link_kind, Some(LinkKind::Reference));
        assert_eq!(event.old_value.as_deref(), Some("Example"));
    });
}

// 8. Deleting a ticket cascades its references.
#[test]
fn test_delete_ticket_cascades_references() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        references
            .create_reference(owner, group_id, ticket_id, reference_request("https://a.com", None))
            .await
            .expect("create reference failed");

        tickets
            .delete_ticket(owner, group_id, ticket_id)
            .await
            .expect("delete ticket failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_references")
            .count_documents(doc! { "ticket_id": ticket_id })
            .await
            .expect("count failed");
        assert_eq!(count, 0, "references were orphaned by ticket deletion");
    });
}

// 9. Deleting a whole group cascades its references.
#[test]
fn test_delete_group_cascades_references() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        references
            .create_reference(owner, group_id, ticket_id, reference_request("https://a.com", None))
            .await
            .expect("create reference failed");

        groups
            .delete_group(owner, group_id)
            .await
            .expect("delete group failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_references")
            .count_documents(doc! { "group_id": group_id })
            .await
            .expect("count failed");
        assert_eq!(count, 0, "references were orphaned by group deletion");
    });
}

// 10. A non-member cannot list a ticket's references.
#[test]
fn test_list_references_non_member_forbidden() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket = seed_ticket(&tickets, group_id, owner).await;
        let ticket_id = ObjectId::parse_str(&ticket.id).unwrap();

        let result = references.list_references(oid(), group_id, ticket_id).await;
        assert_forbidden(result);
    });
}

// 11. Same cross-tenant guard as comments/activity/links: a ticket_id from
// another group 404s.
#[test]
fn test_list_references_with_ticket_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, references, _activity) = setup().await;
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
        let ticket_in_y_id = ObjectId::parse_str(&ticket_in_y.id).unwrap();

        let result = references.list_references(owner_x, group_x_id, ticket_in_y_id).await;
        assert_not_found(result);
    });
}
