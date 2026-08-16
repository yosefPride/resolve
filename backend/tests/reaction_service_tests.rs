use mongodb::{IndexModel, bson::Document, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::comment::service::CommentService;
use resolve::errors::ApiError;
use resolve::group::service::GroupService;
use resolve::reaction::service::ReactionService;
use resolve::ticket::models::{CreateTicketInput, TicketPriority, TicketStatus, UpdateTicketRequest};
use resolve::ticket::repository::TicketRepository;
use resolve::ticket::service::TicketService;

mod support;

const THUMBS_UP: &str = "\u{1F44D}";
const PARTY: &str = "\u{1F389}";

async fn setup() -> (GroupService, TicketService, CommentService, ReactionService) {
    let db = support::shared_client().await.database("resolve_test");

    for collection in [
        "groups",
        "group_members",
        "tickets",
        "counters",
        "comments",
        "comment_reactions",
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

    db.collection::<Document>("comment_reactions")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "comment_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create comment_reactions compound index");

    (
        GroupService::new(&db),
        TicketService::new(&db),
        CommentService::new(&db),
        ReactionService::new(&db),
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

// Same allocation as comment_service_tests.rs's seed_ticket, so the counter
// document exists too.
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

async fn count_reactions(comment_id: ObjectId) -> u64 {
    let db = support::shared_client().await.database("resolve_test");
    db.collection::<Document>("comment_reactions")
        .count_documents(doc! { "comment_id": comment_id })
        .await
        .expect("reaction count failed")
}

// 1. Any group member can react; the response summarizes the new state.
#[test]
fn test_set_reaction_member_succeeds() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        let summary = reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");

        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].emoji, THUMBS_UP);
        assert_eq!(summary[0].count, 1);
        assert!(summary[0].reacted_by_me);
    });
}

// 2. Picking a new emoji replaces the user's existing reaction on this
// comment — the "one reaction per comment per user" rule, not a set.
#[test]
fn test_set_reaction_replaces_previous_emoji_for_same_user() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("first set_reaction failed");
        let summary = reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, PARTY.to_string())
            .await
            .expect("second set_reaction failed");

        assert_eq!(summary.len(), 1, "the old emoji must be gone, not added alongside");
        assert_eq!(summary[0].emoji, PARTY);
        assert_eq!(summary[0].count, 1);
        assert_eq!(count_reactions(comment_id).await, 1);
    });
}

// 3. Setting the same emoji twice is idempotent.
#[test]
fn test_set_reaction_same_emoji_twice_is_idempotent() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("first set_reaction failed");
        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("second set_reaction failed");

        assert_eq!(count_reactions(comment_id).await, 1);
    });
}

// 4. Removing a reaction clears it from the summary entirely.
#[test]
fn test_remove_reaction_clears_it() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");
        let summary = reactions
            .remove_reaction(owner, group_id, ticket_id, comment_id)
            .await
            .expect("remove_reaction failed");

        assert!(summary.is_empty());
        assert_eq!(count_reactions(comment_id).await, 0);
    });
}

// 5. Two different users reacting with the same emoji aggregate into one
// count, and each sees their own reacted_by_me correctly when listing
// through CommentService — proving the enrich_comment wiring, not just
// ReactionService in isolation.
#[test]
fn test_multiple_users_same_emoji_aggregate_and_reacted_by_me_is_per_viewer() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
        let owner = oid();
        let other = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(owner, group_id, other, resolve::group::models::Role::Contributor)
            .await
            .expect("add member failed");
        let ticket_id = seed_ticket(group_id, owner).await;
        let comment = comments
            .create_comment(owner, group_id, ticket_id, "hello".to_string(), None)
            .await
            .expect("create comment failed");
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("owner set_reaction failed");
        reactions
            .set_reaction(other, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("other set_reaction failed");

        let as_owner = comments
            .list_comments(owner, group_id, ticket_id)
            .await
            .expect("list as owner failed");
        assert_eq!(as_owner[0].reactions.len(), 1);
        assert_eq!(as_owner[0].reactions[0].count, 2);
        assert!(as_owner[0].reactions[0].reacted_by_me);

        let as_other = comments
            .list_comments(other, group_id, ticket_id)
            .await
            .expect("list as other failed");
        assert!(as_other[0].reactions[0].reacted_by_me);
    });
}

// 6. A non-member cannot react or unreact.
#[test]
fn test_set_and_remove_reaction_non_member_forbidden() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        let outsider = oid();
        assert_forbidden(
            reactions
                .set_reaction(outsider, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
                .await,
        );
        assert_forbidden(
            reactions
                .remove_reaction(outsider, group_id, ticket_id, comment_id)
                .await,
        );
    });
}

// 7. Reacting to a comment id that doesn't exist 404s.
#[test]
fn test_set_reaction_on_missing_comment_not_found() {
    support::runtime().block_on(async {
        let (groups, _tickets, _comments, reactions) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;

        let result = reactions
            .set_reaction(owner, group_id, ticket_id, oid(), THUMBS_UP.to_string())
            .await;
        assert_not_found(result);
    });
}

// 8. A comment_id from another group/ticket 404s rather than leaking a
// reaction across the tenant boundary — same guard CommentService's
// require_ticket_in_group exists for, proven here via
// CommentRepository::find_by_id's three-field filter.
#[test]
fn test_set_reaction_with_comment_from_another_group_is_rejected() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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

        let ticket_x_id = seed_ticket(group_x_id, owner_x).await;
        let ticket_y_id = seed_ticket(group_y_id, owner_y).await;
        let comment_y = comments
            .create_comment(owner_y, group_y_id, ticket_y_id, "in Y".to_string(), None)
            .await
            .expect("create comment in Y failed");
        let comment_y_id = ObjectId::parse_str(&comment_y.id).unwrap();

        let result = reactions
            .set_reaction(
                owner_x,
                group_x_id,
                ticket_x_id,
                comment_y_id,
                THUMBS_UP.to_string(),
            )
            .await;
        assert_not_found(result);
    });
}

// 9. Reactions are allowed on a closed ticket — deliberately not gated like
// CommentService::create_comment's closed-ticket lock (resolve-emoji-
// reactions-plan: unlike a new comment, a reaction isn't new discussion).
#[test]
fn test_set_reaction_on_closed_ticket_succeeds() {
    support::runtime().block_on(async {
        let (groups, tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

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
            .expect("close ticket failed");

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("reacting on a closed ticket must be allowed");
    });
}

// 10. Hard-deleting a leaf comment clears its reactions too.
#[test]
fn test_delete_leaf_comment_cascades_reactions() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");

        comments
            .delete_comment(owner, group_id, ticket_id, comment_id)
            .await
            .expect("delete comment failed");

        assert_eq!(count_reactions(comment_id).await, 0);
    });
}

// 11. Tombstoning a comment (it has replies) clears its reactions too — a
// deleted comment has nothing left worth reacting to.
#[test]
fn test_tombstoned_comment_cascades_reactions() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
        let owner = oid();
        let group = groups
            .create_group(owner, "G".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let ticket_id = seed_ticket(group_id, owner).await;
        let parent = comments
            .create_comment(owner, group_id, ticket_id, "parent".to_string(), None)
            .await
            .expect("create parent failed");
        let parent_id = ObjectId::parse_str(&parent.id).unwrap();
        comments
            .create_comment(owner, group_id, ticket_id, "reply".to_string(), Some(parent_id))
            .await
            .expect("create reply failed");

        reactions
            .set_reaction(owner, group_id, ticket_id, parent_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");

        comments
            .delete_comment(owner, group_id, ticket_id, parent_id)
            .await
            .expect("delete parent failed");

        assert_eq!(count_reactions(parent_id).await, 0);
    });
}

// 12. Deleting a ticket cascades every reaction on its comments.
#[test]
fn test_delete_ticket_cascades_reactions() {
    support::runtime().block_on(async {
        let (groups, tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");

        tickets
            .delete_ticket(owner, group_id, ticket_id)
            .await
            .expect("delete ticket failed");

        assert_eq!(count_reactions(comment_id).await, 0);
    });
}

// 13. Deleting a whole group cascades every reaction in it.
#[test]
fn test_delete_group_cascades_reactions() {
    support::runtime().block_on(async {
        let (groups, _tickets, comments, reactions) = setup().await;
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
        let comment_id = ObjectId::parse_str(&comment.id).unwrap();

        reactions
            .set_reaction(owner, group_id, ticket_id, comment_id, THUMBS_UP.to_string())
            .await
            .expect("set_reaction failed");

        groups
            .delete_group(owner, group_id)
            .await
            .expect("delete group failed");

        assert_eq!(count_reactions(comment_id).await, 0);
    });
}
