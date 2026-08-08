use mongodb::{IndexModel, bson::Document, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::activity::models::ActivityEventType;
use resolve::activity::service::ActivityService;
use resolve::errors::ApiError;
use resolve::group::models::Role;
use resolve::group::service::GroupService;
use resolve::link::models::{CreateLinkRequest, LinkLabel, RelationType};
use resolve::link::service::LinkService;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse};
use resolve::ticket::service::TicketService;

mod support;

async fn setup() -> (GroupService, TicketService, LinkService, ActivityService) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "groups",
        "group_members",
        "tickets",
        "counters",
        "ticket_links",
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

    db.collection::<Document>("ticket_links")
        .create_index(
            IndexModel::builder()
                .keys(
                    doc! { "group_id": 1, "source_ticket_id": 1, "target_ticket_id": 1, "relation_type": 1 },
                )
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create ticket_links compound index");

    (
        GroupService::new(&db),
        TicketService::new(&db),
        LinkService::new(&db),
        ActivityService::new(&db),
    )
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
}

fn assert_validation<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Validation(_))), "{result:?}");
}

fn assert_conflict<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Conflict(_))), "{result:?}");
}

fn assert_not_found<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
}

async fn seed_ticket(
    tickets: &TicketService,
    group_id: ObjectId,
    created_by: ObjectId,
    title: &str,
) -> TicketResponse {
    tickets
        .create_ticket(
            created_by,
            group_id,
            CreateTicketRequest {
                title: title.to_string(),
                description: "description".to_string(),
                priority: TicketPriority::Low,
            },
        )
        .await
        .expect("create ticket failed")
}

fn link_request(target_id: &str, relation_type: RelationType) -> CreateLinkRequest {
    CreateLinkRequest {
        target_ticket_id: target_id.to_string(),
        relation_type,
    }
}

// 1. Any group member can create a link; the response is resolved from the
// requesting ticket's viewpoint.
#[test]
fn test_create_link_member_succeeds() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        let link = links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await
            .expect("create link failed");

        assert_eq!(link.label, LinkLabel::Blocks);
        assert_eq!(link.other_ticket_id, b.id);
        assert_eq!(link.other_ticket_number, b.ticket_number);
        assert_eq!(link.other_ticket_title, "B");
        assert_eq!(link.created_by, owner.to_hex());
    });
}

// 2. A ticket cannot be linked to itself.
#[test]
fn test_create_link_self_link_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        let result = links
            .create_link(owner, group_id, a_id, link_request(&a.id, RelationType::RelatesTo))
            .await;
        assert_validation(result);
    });
}

// 3. A target ticket that doesn't belong to this group is a validation
// error, not a leak of its existence.
#[test]
fn test_create_link_target_from_another_group_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
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

        let a = seed_ticket(&tickets, group_x_id, owner_x, "A").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b = seed_ticket(&tickets, group_y_id, owner_y, "B").await;

        let result = links
            .create_link(owner_x, group_x_id, a_id, link_request(&b.id, RelationType::RelatesTo))
            .await;
        assert_validation(result);
    });
}

// 4. Adding the exact same (source, target, relation) twice is a conflict.
#[test]
fn test_create_link_exact_duplicate_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await
            .expect("first link failed");

        let result = links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await;
        assert_conflict(result);
    });
}

// 5. RelatesTo is symmetric — creating it from B after it already exists
// from A is also a conflict, not a second document.
#[test]
fn test_create_link_relates_to_symmetric_duplicate_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::RelatesTo))
            .await
            .expect("first link failed");

        let result = links
            .create_link(owner, group_id, b_id, link_request(&a.id, RelationType::RelatesTo))
            .await;
        assert_conflict(result);
    });
}

// 6. Blocks is directional — "A blocks B" and "B blocks A" are different
// assertions, so both are allowed to coexist.
#[test]
fn test_create_link_blocks_reverse_direction_allowed() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await
            .expect("A blocks B failed");
        links
            .create_link(owner, group_id, b_id, link_request(&a.id, RelationType::Blocks))
            .await
            .expect("B blocks A should be allowed alongside A blocks B");
    });
}

// 7. Listing resolves the label from whichever ticket is being viewed.
#[test]
fn test_list_links_resolves_label_from_viewpoint() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await
            .expect("create link failed");

        let from_a = links.list_links(owner, group_id, a_id).await.expect("list from A failed");
        assert_eq!(from_a.len(), 1);
        assert_eq!(from_a[0].label, LinkLabel::Blocks);
        assert_eq!(from_a[0].other_ticket_id, b.id);

        let from_b = links.list_links(owner, group_id, b_id).await.expect("list from B failed");
        assert_eq!(from_b.len(), 1);
        assert_eq!(from_b[0].label, LinkLabel::IsBlockedBy);
        assert_eq!(from_b[0].other_ticket_id, a.id);
    });
}

// 8. A Group Admin (not the link's creator) can still delete it.
#[test]
fn test_delete_link_by_group_admin_non_owner_succeeds() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
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
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        let link = links
            .create_link(
                contributor,
                group_id,
                a_id,
                link_request(&b.id, RelationType::RelatesTo),
            )
            .await
            .expect("create link failed");
        let link_id = ObjectId::parse_str(&link.id).unwrap();

        links
            .delete_link(owner, group_id, a_id, link_id)
            .await
            .expect("group admin delete failed");
    });
}

// 9. A Contributor who neither created the link nor is a Group Admin is
// forbidden from deleting it.
#[test]
fn test_delete_link_by_other_contributor_forbidden() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
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
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        let link = links
            .create_link(author, group_id, a_id, link_request(&b.id, RelationType::RelatesTo))
            .await
            .expect("create link failed");
        let link_id = ObjectId::parse_str(&link.id).unwrap();

        let result = links.delete_link(other, group_id, a_id, link_id).await;
        assert_forbidden(result);
    });
}

// 10. Creating a link logs an activity event on both tickets, each phrased
// from its own viewpoint.
#[test]
fn test_create_link_records_activity_on_both_tickets() {
    support::runtime().block_on(async {
        let (groups, tickets, links, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Blocks))
            .await
            .expect("create link failed");

        let a_activity = activity
            .list_activity(owner, group_id, a_id)
            .await
            .expect("list activity for A failed");
        let a_event = a_activity
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkAdded)
            .expect("link_added missing on A");
        assert_eq!(a_event.new_value.as_deref(), Some(format!("blocks #{}", b.ticket_number).as_str()));

        let b_activity = activity
            .list_activity(owner, group_id, b_id)
            .await
            .expect("list activity for B failed");
        let b_event = b_activity
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkAdded)
            .expect("link_added missing on B");
        assert_eq!(
            b_event.new_value.as_deref(),
            Some(format!("is blocked by #{}", a.ticket_number).as_str())
        );
    });
}

// 11. Deleting a link logs a link_removed event on both tickets too.
#[test]
fn test_delete_link_records_activity_on_both_tickets() {
    support::runtime().block_on(async {
        let (groups, tickets, links, activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        let link = links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::Duplicates))
            .await
            .expect("create link failed");
        let link_id = ObjectId::parse_str(&link.id).unwrap();

        links
            .delete_link(owner, group_id, a_id, link_id)
            .await
            .expect("delete link failed");

        let a_activity = activity
            .list_activity(owner, group_id, a_id)
            .await
            .expect("list activity for A failed");
        let a_event = a_activity
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkRemoved)
            .expect("link_removed missing on A");
        assert_eq!(
            a_event.old_value.as_deref(),
            Some(format!("duplicates #{}", b.ticket_number).as_str())
        );

        let b_activity = activity
            .list_activity(owner, group_id, b_id)
            .await
            .expect("list activity for B failed");
        let b_event = b_activity
            .iter()
            .find(|e| e.event_type == ActivityEventType::LinkRemoved)
            .expect("link_removed missing on B");
        assert_eq!(
            b_event.old_value.as_deref(),
            Some(format!("is duplicated by #{}", a.ticket_number).as_str())
        );
    });
}

// 12. Deleting either ticket in a link removes the link entirely.
#[test]
fn test_delete_ticket_cascades_links() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();
        let b_id = ObjectId::parse_str(&b.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::RelatesTo))
            .await
            .expect("create link failed");

        tickets
            .delete_ticket(owner, group_id, b_id)
            .await
            .expect("delete ticket B failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_links")
            .count_documents(doc! { "$or": [ { "source_ticket_id": a_id }, { "target_ticket_id": a_id } ] })
            .await
            .expect("count failed");
        assert_eq!(count, 0, "link was orphaned by the other ticket's deletion");
    });
}

// 13. Deleting a whole group cascades every link in it.
#[test]
fn test_delete_group_cascades_links() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let b = seed_ticket(&tickets, group_id, owner, "B").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        links
            .create_link(owner, group_id, a_id, link_request(&b.id, RelationType::RelatesTo))
            .await
            .expect("create link failed");

        groups
            .delete_group(owner, group_id)
            .await
            .expect("delete group failed");

        let db = support::shared_client().await.database("resolve_test");
        let count = db
            .collection::<Document>("ticket_links")
            .count_documents(doc! { "group_id": group_id })
            .await
            .expect("count failed");
        assert_eq!(count, 0, "links were orphaned by group deletion");
    });
}

// 14. A non-member cannot list a ticket's links.
#[test]
fn test_list_links_non_member_forbidden() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let a = seed_ticket(&tickets, group_id, owner, "A").await;
        let a_id = ObjectId::parse_str(&a.id).unwrap();

        let result = links.list_links(oid(), group_id, a_id).await;
        assert_forbidden(result);
    });
}

// 15. Same cross-tenant guard as comments/activity: a ticket_id from another
// group 404s on the list path.
#[test]
fn test_list_links_with_ticket_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, links, _activity) = setup().await;
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

        let ticket_in_y = seed_ticket(&tickets, group_y_id, owner_y, "Y-ticket").await;
        let ticket_in_y_id = ObjectId::parse_str(&ticket_in_y.id).unwrap();

        let result = links.list_links(owner_x, group_x_id, ticket_in_y_id).await;
        assert_not_found(result);
    });
}
