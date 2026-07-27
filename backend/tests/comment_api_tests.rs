use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::comment::models::{CommentResponse, CreateCommentRequest};
use resolve::config::Config;
use resolve::group::models::{AddMemberRequest, CreateGroupRequest, GroupResponse, Role};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{
    CreateTicketRequest, TicketPriority, TicketResponse, TicketStatus, UpdateTicketRequest,
};
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

// Same shared-db convention as tests/ticket_api_tests.rs's setup_db().
async fn setup_db() -> (Database, String) {
    let db = support::shared_client().await.database("resolve_test");
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");

    db.collection::<mongodb::bson::Document>("users")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "email": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create email index");

    db.collection::<mongodb::bson::Document>("group_members")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create group_members compound index");

    (db, uri)
}

fn build_app_state(db: Database, uri: String) -> web::Data<AppState> {
    web::Data::new(AppState {
        db,
        config: Config {
            mongo_uri: uri,
            jwt_secret: TEST_JWT_SECRET.to_string(),
            cookie_secure: false,
            frontend_origin: "http://localhost:5173".to_string(),
        },
    })
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
}

fn register_request(prefix: &str) -> actix_web::test::TestRequest {
    actix_test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(&RegisterRequest {
            email: unique_email(prefix),
            password: "password123".to_string(),
            name: prefix.to_string(),
        })
}

fn auth_header(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

// Mirrors ticket_api_tests.rs's cleanup: leftover comment docs are keyed to a
// now-deleted group's unique id, so no other test in the shared db can observe
// them.
async fn cleanup(
    group_repo: &GroupRepository,
    user_repo: &UserRepository,
    group_id: ObjectId,
    users: &[&AuthResponse],
) {
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    for user in users {
        user_repo
            .delete(ObjectId::parse_str(&user.user.id).unwrap())
            .await
            .ok();
    }
}

// 1. The full happy path over HTTP: create a root comment, reply to it
// (threading), list them back oldest-first, and confirm the reply's parent
// link survives the round trip.
#[test]
fn test_create_and_list_comments_with_threading() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-contrib").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Comment API Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/groups/{}/users", group.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&AddMemberRequest {
                    user_id: contributor.user.id.clone(),
                    role: Role::Contributor,
                })
                .to_request(),
        )
        .await;

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Discussed ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let comments_uri = format!(
            "/api/v1/groups/{}/tickets/{}/comments",
            group.id, ticket.id
        );

        // A Contributor may comment — commenting is open to any group member.
        let root_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&contributor.jwt))
                .set_json(&CreateCommentRequest {
                    content: "root comment".to_string(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(root_resp.status(), 201);
        let root: CommentResponse = actix_test::read_body_json(root_resp).await;
        assert!(root.parent_comment_id.is_none());
        assert!(!root.is_deleted);

        let reply_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: "a reply".to_string(),
                    parent_comment_id: Some(root.id.clone()),
                })
                .to_request(),
        )
        .await;
        assert_eq!(reply_resp.status(), 201);
        let reply: CommentResponse = actix_test::read_body_json(reply_resp).await;
        assert_eq!(reply.parent_comment_id.as_deref(), Some(root.id.as_str()));

        let list_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(list_resp.status(), 200);
        let listing: Vec<CommentResponse> = actix_test::read_body_json(list_resp).await;
        assert_eq!(listing.len(), 2);
        assert_eq!(listing[0].content, "root comment");
        assert_eq!(listing[1].content, "a reply");
        // The author's display name is resolved server-side.
        assert_eq!(listing[0].user_name, "cmt-contrib");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &contributor]).await;
    });
}

// 2. Content validation over HTTP: empty content and over-length content are
// both 400, and — the regression that matters — 2000 Hebrew characters are
// ACCEPTED. A byte-based (.len()) check would wrongly reject those, since each
// is 2 bytes in UTF-8. This is the path the browser actually exercises.
#[test]
fn test_comment_content_validation_over_http() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-valid").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Validation Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Validated ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let comments_uri = format!(
            "/api/v1/groups/{}/tickets/{}/comments",
            group.id, ticket.id
        );

        let blank_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: "   ".to_string(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(blank_resp.status(), 400, "blank content must be rejected");

        let too_long_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: "a".repeat(2001),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(too_long_resp.status(), 400, "2001 chars must be rejected");

        // 2000 Hebrew chars = ~4000 bytes. Accepted iff the limit counts
        // characters, not bytes.
        let hebrew = "א".repeat(2000);
        let hebrew_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: hebrew.clone(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(
            hebrew_resp.status(),
            201,
            "2000 Hebrew chars must be accepted — a byte-based limit would reject them"
        );
        let stored: CommentResponse = actix_test::read_body_json(hebrew_resp).await;
        assert_eq!(
            stored.content.chars().count(),
            2000,
            "Hebrew content must round-trip intact"
        );
        assert_eq!(stored.content, hebrew);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// 3. Deletion over HTTP: a non-owner Contributor gets 403; a leaf comment is
// removed outright (204); a comment with a reply is tombstoned rather than
// removed, so the reply keeps a valid parent.
#[test]
fn test_delete_comment_permissions_and_tombstone_over_http() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-del-owner").to_request()).await,
        )
        .await;
        let author: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-del-author").to_request()).await,
        )
        .await;
        let bystander: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-del-other").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Delete Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        for member in [&author, &bystander] {
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/users", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&AddMemberRequest {
                        user_id: member.user.id.clone(),
                        role: Role::Contributor,
                    })
                    .to_request(),
            )
            .await;
        }

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Deletion ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let comments_uri = format!(
            "/api/v1/groups/{}/tickets/{}/comments",
            group.id, ticket.id
        );

        let root: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&comments_uri)
                    .insert_header(auth_header(&author.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "author's comment".to_string(),
                        parent_comment_id: None,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        // A Contributor who is neither the author nor a Group Admin: 403.
        let forbidden_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("{comments_uri}/{}", root.id))
                .insert_header(auth_header(&bystander.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(forbidden_resp.status(), 403);

        // Give the root a reply, so deleting the root must tombstone it.
        let reply: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&comments_uri)
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "a reply".to_string(),
                        parent_comment_id: Some(root.id.clone()),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let tombstone_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("{comments_uri}/{}", root.id))
                .insert_header(auth_header(&author.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(tombstone_resp.status(), 204);

        let listing: Vec<CommentResponse> = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::get()
                    .uri(&comments_uri)
                    .insert_header(auth_header(&owner.jwt))
                    .to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(listing.len(), 2, "the tombstoned parent must still be listed");
        let stored_root = listing
            .iter()
            .find(|c| c.id == root.id)
            .expect("root still present");
        assert!(stored_root.is_deleted);
        assert_eq!(stored_root.content, "[comment deleted]");

        // The reply is a leaf, so deleting it removes it outright.
        let leaf_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("{comments_uri}/{}", reply.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(leaf_resp.status(), 204);

        let listing: Vec<CommentResponse> = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::get()
                    .uri(&comments_uri)
                    .insert_header(auth_header(&owner.jwt))
                    .to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(listing.len(), 1, "the leaf reply must be hard-deleted");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(
            &group_repo,
            &user_repo,
            group_id,
            &[&owner, &author, &bystander],
        )
        .await;
    });
}

// 4. Group isolation over HTTP — the regression guard for the cross-tenant
// leak found in live testing. A member of group X passing group X's id with a
// ticket_id belonging to group Y must be rejected on both create and list, and
// a non-member of Y must not reach Y's thread at all.
#[test]
fn test_cross_group_ticket_id_is_rejected_over_http() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner_x: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-iso-x").to_request()).await,
        )
        .await;
        let owner_y: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-iso-y").to_request()).await,
        )
        .await;

        let group_x: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner_x.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Isolation X".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let group_y: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner_y.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Isolation Y".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let ticket_y: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group_y.id))
                    .insert_header(auth_header(&owner_y.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Y's private ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        // owner_x is a legitimate Group Admin of X — the only thing wrong is
        // that the ticket belongs to Y.
        let cross_uri = format!(
            "/api/v1/groups/{}/tickets/{}/comments",
            group_x.id, ticket_y.id
        );

        let cross_create = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&cross_uri)
                .insert_header(auth_header(&owner_x.jwt))
                .set_json(&CreateCommentRequest {
                    content: "cross-group injection".to_string(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(
            cross_create.status(),
            404,
            "a ticket from another group must not accept comments under this group"
        );

        let cross_list = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&cross_uri)
                .insert_header(auth_header(&owner_x.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(cross_list.status(), 404);

        // And a straight non-member read of Y's own thread is 403.
        let outsider = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/comments",
                    group_y.id, ticket_y.id
                ))
                .insert_header(auth_header(&owner_x.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(outsider.status(), 403);

        // Y's own thread stayed clean — nothing leaked into it.
        let y_listing: Vec<CommentResponse> = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::get()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group_y.id, ticket_y.id
                    ))
                    .insert_header(auth_header(&owner_y.jwt))
                    .to_request(),
            )
            .await,
        )
        .await;
        assert!(y_listing.is_empty());

        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_x.id).unwrap(),
            &[&owner_x],
        )
        .await;
        cleanup(
            &group_repo,
            &user_repo,
            ObjectId::parse_str(&group_y.id).unwrap(),
            &[&owner_y],
        )
        .await;
    });
}

// 5. A closed ticket is read-only for discussion: new comments are rejected
// with 409, but the existing thread stays readable and its comments deletable.
#[test]
fn test_closed_ticket_rejects_new_comments_over_http() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("cmt-closed").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Closed Ticket Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Soon to be closed".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let comments_uri = format!(
            "/api/v1/groups/{}/tickets/{}/comments",
            group.id, ticket.id
        );

        let before: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&comments_uri)
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "said while open".to_string(),
                        parent_comment_id: None,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let close_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::patch()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: None,
                    status: Some(TicketStatus::Closed),
                })
                .to_request(),
        )
        .await;
        assert_eq!(close_resp.status(), 200);

        let rejected = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: "said after closing".to_string(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(
            rejected.status(),
            409,
            "a closed ticket must not accept new comments"
        );

        // Reading the thread still works...
        let list_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&comments_uri)
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(list_resp.status(), 200);
        let listing: Vec<CommentResponse> = actix_test::read_body_json(list_resp).await;
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].content, "said while open");

        // ...and so does deleting an existing comment.
        let delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("{comments_uri}/{}", before.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(delete_resp.status(), 204);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}
