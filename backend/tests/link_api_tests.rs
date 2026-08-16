use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::group::models::{AddMemberRequest, CreateGroupRequest, GroupResponse, Role};
use resolve::group::repository::GroupRepository;
use resolve::link::models::{CreateLinkRequest, LinkLabel, RelationType, TicketLinkResponse};
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{CreateTicketRequest, TicketPriority, TicketResponse};
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

    db.collection::<mongodb::bson::Document>("ticket_links")
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

// The full happy path over HTTP: create two tickets, link them, list the
// link back from both sides (label flips with viewpoint), then delete it.
#[test]
fn test_create_list_and_delete_link_over_http() {
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
            actix_test::call_service(&app, register_request("link-owner").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Link API Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let ticket_a: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Ticket A".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let ticket_b: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Ticket B".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let create_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/links",
                    group.id, ticket_a.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateLinkRequest {
                    target_ticket_id: ticket_b.id.clone(),
                    relation_type: RelationType::Blocks,
                })
                .to_request(),
        )
        .await;
        assert_eq!(create_resp.status(), 201);
        let link: TicketLinkResponse = actix_test::read_body_json(create_resp).await;
        assert_eq!(link.label, LinkLabel::Blocks);

        let list_from_b = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/links",
                    group.id, ticket_b.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(list_from_b.status(), 200);
        let listing: Vec<TicketLinkResponse> = actix_test::read_body_json(list_from_b).await;
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].label, LinkLabel::IsBlockedBy);
        assert_eq!(listing[0].other_ticket_id, ticket_a.id);

        let delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/links/{}",
                    group.id, ticket_a.id, link.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(delete_resp.status(), 204);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}

// A Contributor who didn't create the link and isn't a Group Admin gets 403
// on delete.
#[test]
fn test_delete_link_forbidden_for_non_owner_contributor_over_http() {
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
            actix_test::call_service(&app, register_request("link-del-owner").to_request()).await,
        )
        .await;
        let other: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("link-del-other").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Link Delete Group".to_string(),
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
                    user_id: other.user.id.clone(),
                    role: Role::Contributor,
                })
                .to_request(),
        )
        .await;

        let ticket_a: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Ticket A".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;
        let ticket_b: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Ticket B".to_string(),
                        description: "d".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let create_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/links",
                    group.id, ticket_a.id
                ))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&CreateLinkRequest {
                    target_ticket_id: ticket_b.id.clone(),
                    relation_type: RelationType::RelatesTo,
                })
                .to_request(),
        )
        .await;
        let link: TicketLinkResponse = actix_test::read_body_json(create_resp).await;

        let delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!(
                    "/api/v1/groups/{}/tickets/{}/links/{}",
                    group.id, ticket_a.id, link.id
                ))
                .insert_header(auth_header(&other.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(delete_resp.status(), 403);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &other]).await;
    });
}
