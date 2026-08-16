use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::comment::models::{CommentResponse, CreateCommentRequest};
use resolve::group::models::{CreateGroupRequest, GroupResponse};
use resolve::group::repository::GroupRepository;
use resolve::reaction::models::{ReactionSummary, SetReactionRequest};
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse, TicketStatus, UpdateTicketRequest};
use resolve::user::repository::UserRepository;

mod support;

// Same shared-db convention as tests/comment_api_tests.rs's setup_db().
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

    db.collection::<mongodb::bson::Document>("comment_reactions")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "comment_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create comment_reactions compound index");

    (db, uri)
}

fn build_app_state(db: Database, uri: String) -> web::Data<AppState> {
    web::Data::new(AppState {
        db,
        config: support::test_config(uri),
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

fn reaction_uri(group_id: &str, ticket_id: &str, comment_id: &str) -> String {
    format!("/api/v1/groups/{group_id}/tickets/{ticket_id}/comments/{comment_id}/reactions")
}

// 1. Full happy path: PUT sets a reaction, the comment listing reflects it
// with the correct count and reacted_by_me, then DELETE clears it again.
#[test]
fn test_set_and_remove_reaction_over_http() {
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
            actix_test::call_service(&app, register_request("rxn-happy").to_request()).await,
        )
        .await;
        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Reaction Happy Group".to_string(),
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
                        title: "Reactable ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let comment: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group.id, ticket.id
                    ))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "react to me".to_string(),
                        parent_comment_id: None,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let reactions_uri = reaction_uri(&group.id, &ticket.id, &comment.id);

        let set_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reactions_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "\u{1F44D}".to_string(),
                })
                .to_request(),
        )
        .await;
        assert_eq!(set_resp.status(), 200);
        let summary: Vec<ReactionSummary> = actix_test::read_body_json(set_resp).await;
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].count, 1);
        assert!(summary[0].reacted_by_me);

        let listing: Vec<CommentResponse> = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::get()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group.id, ticket.id
                    ))
                    .insert_header(auth_header(&owner.jwt))
                    .to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(listing[0].reactions.len(), 1);
        assert_eq!(listing[0].reactions[0].emoji, "\u{1F44D}");

        let remove_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&reactions_uri)
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(remove_resp.status(), 200);
        let summary: Vec<ReactionSummary> = actix_test::read_body_json(remove_resp).await;
        assert!(summary.is_empty());

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// 2. A blank or over-length emoji is rejected with 400.
#[test]
fn test_reaction_validation_over_http() {
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
            actix_test::call_service(&app, register_request("rxn-valid").to_request()).await,
        )
        .await;
        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Reaction Validation Group".to_string(),
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
                        title: "Reactable ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let comment: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group.id, ticket.id
                    ))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "react to me".to_string(),
                        parent_comment_id: None,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let reactions_uri = reaction_uri(&group.id, &ticket.id, &comment.id);

        let blank_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reactions_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "   ".to_string(),
                })
                .to_request(),
        )
        .await;
        assert_eq!(blank_resp.status(), 400, "blank emoji must be rejected");

        let too_long_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reactions_uri)
                .insert_header(auth_header(&owner.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "a".repeat(9),
                })
                .to_request(),
        )
        .await;
        assert_eq!(too_long_resp.status(), 400, "over-length emoji must be rejected");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// 3. Reactions stay allowed on a closed ticket — deliberately not gated the
// way new comments are (resolve-emoji-reactions-plan).
#[test]
fn test_reaction_on_closed_ticket_allowed_over_http() {
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
            actix_test::call_service(&app, register_request("rxn-closed").to_request()).await,
        )
        .await;
        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Reaction Closed Group".to_string(),
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
                        title: "Soon closed".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let comment: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group.id, ticket.id
                    ))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "react to me".to_string(),
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

        let react_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reaction_uri(&group.id, &ticket.id, &comment.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "\u{1F44D}".to_string(),
                })
                .to_request(),
        )
        .await;
        assert_eq!(
            react_resp.status(),
            200,
            "reacting on a closed ticket must still be allowed"
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// 4. Reacting to a comment id that doesn't exist 404s.
#[test]
fn test_reaction_on_missing_comment_not_found_over_http() {
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
            actix_test::call_service(&app, register_request("rxn-missing").to_request()).await,
        )
        .await;
        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Reaction Missing Group".to_string(),
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
                        title: "Reactable ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reaction_uri(&group.id, &ticket.id, &ObjectId::new().to_hex()))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "\u{1F44D}".to_string(),
                })
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// 5. A non-member cannot react.
#[test]
fn test_reaction_non_member_forbidden_over_http() {
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
            actix_test::call_service(&app, register_request("rxn-outsider").to_request()).await,
        )
        .await;
        let outsider: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("rxn-outsider2").to_request()).await,
        )
        .await;
        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Reaction Outsider Group".to_string(),
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
                        title: "Reactable ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let comment: CommentResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!(
                        "/api/v1/groups/{}/tickets/{}/comments",
                        group.id, ticket.id
                    ))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateCommentRequest {
                        content: "react to me".to_string(),
                        parent_comment_id: None,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::put()
                .uri(&reaction_uri(&group.id, &ticket.id, &comment.id))
                .insert_header(auth_header(&outsider.jwt))
                .set_json(&SetReactionRequest {
                    emoji: "\u{1F44D}".to_string(),
                })
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 403);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &outsider]).await;
    });
}
