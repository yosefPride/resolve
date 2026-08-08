use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::activity::models::{ActivityEventType, TicketActivityResponse};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::comment::models::CreateCommentRequest;
use resolve::group::models::{CreateGroupRequest, GroupResponse};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse, UpdateTicketRequest};
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

// The full happy path over HTTP: creating a ticket, editing it, and adding a
// comment each land an entry in the Activity feed, newest-first, with the
// actor's live display name resolved server-side.
#[test]
fn test_activity_feed_over_http() {
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
            actix_test::call_service(&app, register_request("act-owner").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Activity API Group".to_string(),
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
                        title: "Tracked ticket".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        actix_test::call_service(
            &app,
            actix_test::TestRequest::patch()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: Some(TicketPriority::High),
                    status: None,
                })
                .to_request(),
        )
        .await;

        actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/comments",
                    group.id, ticket.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateCommentRequest {
                    content: "first comment".to_string(),
                    parent_comment_id: None,
                })
                .to_request(),
        )
        .await;

        let activity_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/activity",
                    group.id, ticket.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(activity_resp.status(), 200);
        let entries: Vec<TicketActivityResponse> = actix_test::read_body_json(activity_resp).await;

        assert_eq!(entries.len(), 3, "created + priority change + comment added");
        assert_eq!(entries[0].event_type, ActivityEventType::CommentAdded);
        assert_eq!(entries[1].event_type, ActivityEventType::PriorityChanged);
        assert_eq!(entries[2].event_type, ActivityEventType::TicketCreated);
        assert_eq!(entries[1].old_value.as_deref(), Some("low"));
        assert_eq!(entries[1].new_value.as_deref(), Some("high"));
        assert_eq!(entries[2].actor_name, "act-owner");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// A non-member gets 403, not the activity feed.
#[test]
fn test_activity_feed_forbidden_for_non_member_over_http() {
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
            actix_test::call_service(&app, register_request("act-forb-owner").to_request()).await,
        )
        .await;
        let outsider: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("act-forb-out").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Forbidden Activity Group".to_string(),
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
                        title: "Private ticket".to_string(),
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
            actix_test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/activity",
                    group.id, ticket.id
                ))
                .insert_header(auth_header(&outsider.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 403);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &outsider]).await;
    });
}
