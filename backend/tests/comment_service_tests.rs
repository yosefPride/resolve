use mongodb::{IndexModel, bson::Document, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::comment::service::CommentService;
use resolve::errors::ApiError;
use resolve::group::models::Role;
use resolve::group::service::GroupService;
use resolve::ticket::models::{CreateTicketInput, TicketPriority, TicketStatus, UpdateTicketRequest};
use resolve::ticket::repository::TicketRepository;
use resolve::ticket::service::TicketService;

mod support;

async fn setup() -> (GroupService, TicketService, CommentService) {
    let db = support::shared_client().await.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    for collection in ["groups", "group_members", "tickets", "counters", "comments"] {
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

fn assert_not_found<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
}

fn assert_conflict<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Conflict(_))), "{result:?}");
}

// Allocates a ticket_number the same way TicketService does, so the counter
// document exists too (matches ticket_api_tests.rs's seed_ticket).
async fn seed_ticket(group_id: ObjectId, created_by: ObjectId) -> ObjectId {
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
            title: "a ticket".to_string(),
            description: "description".to_string(),
            priority: TicketPriority::Low,
            created_by,
        })
        .await
        .expect("ticket insert failed");
    ticket.id.expect("insert_ticket always returns an id")
}

async fn count_comments(ticket_id: ObjectId) -> u64 {
    let db = support::shared_client().await.database("resolve_test");
    db.collection::<Document>("comments")
        .count_documents(doc! { "ticket_id": ticket_id })
        .await
        .expect("comment count failed")
}

// 1. Any group member can comment on a ticket.
#[test]
fn test_create_comment_member_succeeds() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let comment = comments
            .create_comment(owner, group_id, ticket_id, "hello".to_string(), None)
            .await
            .expect("create comment failed");
        assert_eq!(comment.content, "hello");
        assert!(!comment.is_deleted);
        assert!(comment.parent_comment_id.is_none());
    });
}

// 2. A non-member cannot comment.
#[test]
fn test_create_comment_non_member_forbidden() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let result = comments
            .create_comment(oid(), group_id, ticket_id, "hi".to_string(), None)
            .await;
        assert_forbidden(result);
    });
}

// 3. Replying to a comment that belongs to a *different* ticket is rejected —
// a reply's parent must be a comment on the same ticket.
#[test]
fn test_reply_to_comment_from_other_ticket_is_rejected() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_a = seed_ticket(group_id, owner).await;
        let ticket_b = seed_ticket(group_id, owner).await;

        let root = comments
            .create_comment(owner, group_id, ticket_a, "root".to_string(), None)
            .await
            .expect("create root failed");
        let parent_id = ObjectId::parse_str(&root.id).unwrap();

        let result = comments
            .create_comment(owner, group_id, ticket_b, "reply".to_string(), Some(parent_id))
            .await;
        assert_validation(result);
    });
}

// 4. Comments come back oldest-first, with no pagination.
#[test]
fn test_list_comments_returns_oldest_first() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        comments
            .create_comment(owner, group_id, ticket_id, "first".to_string(), None)
            .await
            .expect("create first failed");
        comments
            .create_comment(owner, group_id, ticket_id, "second".to_string(), None)
            .await
            .expect("create second failed");

        let list = comments
            .list_comments(owner, group_id, ticket_id)
            .await
            .expect("list failed");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "first");
        assert_eq!(list[1].content, "second");
    });
}

// 5. Deleting a leaf comment (no replies) removes it entirely.
#[test]
fn test_delete_leaf_comment_hard_deletes() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let comment = comments
            .create_comment(owner, group_id, ticket_id, "bye".to_string(), None)
            .await
            .expect("create failed");
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        comments
            .delete_comment(owner, group_id, ticket_id, comment_id)
            .await
            .expect("delete failed");

        let list = comments
            .list_comments(owner, group_id, ticket_id)
            .await
            .expect("list failed");
        assert!(
            list.is_empty(),
            "a leaf comment should be hard-deleted, not tombstoned"
        );
    });
}

// 6. Deleting a comment that has replies tombstones it instead of removing
// it, so the reply keeps a valid parent to point at.
#[test]
fn test_delete_comment_with_replies_is_tombstoned_not_removed() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let root = comments
            .create_comment(owner, group_id, ticket_id, "root".to_string(), None)
            .await
            .expect("create root failed");
        let root_id = ObjectId::parse_str(&root.id).unwrap();
        comments
            .create_comment(owner, group_id, ticket_id, "reply".to_string(), Some(root_id))
            .await
            .expect("create reply failed");

        comments
            .delete_comment(owner, group_id, ticket_id, root_id)
            .await
            .expect("delete failed");

        let list = comments
            .list_comments(owner, group_id, ticket_id)
            .await
            .expect("list failed");
        assert_eq!(
            list.len(),
            2,
            "the tombstoned parent must still be listed for the reply to render against"
        );
        let tombstoned = list
            .iter()
            .find(|c| c.id == root.id)
            .expect("root comment still present");
        assert!(tombstoned.is_deleted);
        assert_eq!(tombstoned.content, "[comment deleted]");
    });
}

// 7. A Group Admin (not the comment's author) can still delete it.
#[test]
fn test_delete_comment_by_group_admin_non_owner_succeeds() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
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
        let ticket_id = seed_ticket(group_id, owner).await;

        let comment = comments
            .create_comment(contributor, group_id, ticket_id, "mine".to_string(), None)
            .await
            .expect("create failed");
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        comments
            .delete_comment(owner, group_id, ticket_id, comment_id)
            .await
            .expect("group admin delete failed");
    });
}

// 8. A Contributor who neither authored the comment nor is a Group Admin is
// forbidden from deleting it.
#[test]
fn test_delete_comment_by_other_contributor_forbidden() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
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
        let ticket_id = seed_ticket(group_id, owner).await;

        let comment = comments
            .create_comment(author, group_id, ticket_id, "mine".to_string(), None)
            .await
            .expect("create failed");
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        let result = comments
            .delete_comment(other, group_id, ticket_id, comment_id)
            .await;
        assert_forbidden(result);
    });
}

// 9. Deleting a ticket cascades its comments (regardless of tombstone state).
#[test]
fn test_delete_ticket_cascades_comments() {
    support::runtime().block_on(async {
        let (groups, tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        comments
            .create_comment(owner, group_id, ticket_id, "one".to_string(), None)
            .await
            .expect("create c1 failed");
        comments
            .create_comment(owner, group_id, ticket_id, "two".to_string(), None)
            .await
            .expect("create c2 failed");
        assert_eq!(count_comments(ticket_id).await, 2);

        tickets
            .delete_ticket(owner, group_id, ticket_id)
            .await
            .expect("delete ticket failed");

        assert_eq!(
            count_comments(ticket_id).await,
            0,
            "comments were orphaned by ticket deletion"
        );
    });
}

// 10a. Regression guard for a real cross-tenant leak found during live
// testing: a member of group X passing group X's id together with a ticket_id
// that actually belongs to group Y used to have their comment accepted and
// recorded under group X. Membership in the path's group is not enough — the
// ticket must genuinely belong to that group.
#[test]
fn test_create_comment_with_ticket_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
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

        let ticket_in_y = seed_ticket(group_y_id, owner_y).await;

        // owner_x IS a legitimate member of group_x — the only thing wrong
        // here is that the ticket lives in group_y.
        let result = comments
            .create_comment(
                owner_x,
                group_x_id,
                ticket_in_y,
                "cross-group injection".to_string(),
                None,
            )
            .await;
        assert_not_found(result);

        // And nothing was written under group_x as a side effect.
        assert_eq!(count_comments(ticket_in_y).await, 0);
    });
}

// 10b. Same isolation guarantee on the read path.
#[test]
fn test_list_comments_with_ticket_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
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

        let ticket_in_y = seed_ticket(group_y_id, owner_y).await;
        comments
            .create_comment(owner_y, group_y_id, ticket_in_y, "secret".to_string(), None)
            .await
            .expect("seed comment in Y failed");

        let result = comments
            .list_comments(owner_x, group_x_id, ticket_in_y)
            .await;
        assert_not_found(result);
    });
}

// 10c. A closed ticket is read-only for discussion: no new comments.
#[test]
fn test_create_comment_on_closed_ticket_is_rejected() {
    support::runtime().block_on(async {
        let (groups, tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

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
            .expect("closing the ticket failed");

        let result = comments
            .create_comment(owner, group_id, ticket_id, "too late".to_string(), None)
            .await;
        assert_conflict(result);
    });
}

// 10d. ...but the existing thread stays fully readable, and comments on it can
// still be deleted, after the ticket is closed.
#[test]
fn test_closed_ticket_comments_remain_readable_and_deletable() {
    support::runtime().block_on(async {
        let (groups, tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let before = comments
            .create_comment(owner, group_id, ticket_id, "before closing".to_string(), None)
            .await
            .expect("create comment failed");
        let before_id = ObjectId::parse_str(&before.id).unwrap();

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
            .expect("closing the ticket failed");

        let listing = comments
            .list_comments(owner, group_id, ticket_id)
            .await
            .expect("listing a closed ticket's comments must still work");
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].content, "before closing");

        comments
            .delete_comment(owner, group_id, ticket_id, before_id)
            .await
            .expect("deleting a comment on a closed ticket must still work");
    });
}

// 11. Deleting a whole group cascades its comments too.
#[test]
fn test_delete_group_cascades_comments() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        comments
            .create_comment(owner, group_id, ticket_id, "one".to_string(), None)
            .await
            .expect("create failed");
        assert_eq!(count_comments(ticket_id).await, 1);

        groups
            .delete_group(owner, group_id)
            .await
            .expect("delete group failed");

        assert_eq!(
            count_comments(ticket_id).await,
            0,
            "comments were orphaned by group deletion"
        );
    });
}
