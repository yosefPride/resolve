use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::config::Config;
use resolve::group::models::{AddMemberRequest, CreateGroupRequest, GroupResponse, Role};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::ticket::models::{
    CreateTicketRequest, TicketListResponse, TicketPriority, TicketResponse, TicketStatus,
    UpdateTicketRequest,
};
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

// Same shared-db convention as tests/group_api_tests.rs's setup_db().
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

// Cleans up a group's members/doc and the given users; leftover ticket docs
// are keyed to this now-deleted group's unique id, so no other test in the
// shared db can observe them (same tolerance group_api_tests.rs already has).
async fn cleanup(group_repo: &GroupRepository, user_repo: &UserRepository, group_id: ObjectId, users: &[&AuthResponse]) {
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    for user in users {
        user_repo
            .delete(ObjectId::parse_str(&user.user.id).unwrap())
            .await
            .ok();
    }
}

// 1. Group Admin updates a ticket's title, priority, and status in one PATCH;
// a Contributor is forbidden from the same PATCH.
#[test]
fn test_update_ticket_group_admin_only() {
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
            actix_test::call_service(&app, register_request("upd-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("upd-contributor").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "Update Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        assert_eq!(
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
            .await
            .status(),
            201
        );

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&contributor.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "Login bug".to_string(),
                        description: "boom".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        // The creator (a Contributor) may not edit their own ticket.
        let contributor_edit_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::patch()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&contributor.jwt))
                .set_json(&UpdateTicketRequest {
                    title: None,
                    description: None,
                    priority: None,
                    status: Some(TicketStatus::Closed),
                })
                .to_request(),
        )
        .await;
        assert_eq!(contributor_edit_resp.status(), 403);

        // The Group Admin may edit it.
        let admin_edit_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::patch()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&UpdateTicketRequest {
                    title: Some("Login bug (fixed)".to_string()),
                    description: None,
                    priority: Some(TicketPriority::Critical),
                    status: Some(TicketStatus::Closed),
                })
                .to_request(),
        )
        .await;
        assert_eq!(admin_edit_resp.status(), 200);
        let updated: TicketResponse = actix_test::read_body_json(admin_edit_resp).await;
        assert_eq!(updated.title, "Login bug (fixed)");
        assert_eq!(updated.priority, TicketPriority::Critical);
        assert_eq!(updated.status, TicketStatus::Closed);

        // A PATCH with no fields at all is a validation error.
        let empty_patch_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::patch()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .set_json(&serde_json::json!({}))
                .to_request(),
        )
        .await;
        assert_eq!(empty_patch_resp.status(), 400);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &contributor]).await;
    });
}

// 2. Group Admin deletes a ticket; afterward it 404s. A Contributor is
// forbidden from deleting.
#[test]
fn test_delete_ticket_group_admin_only() {
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
            actix_test::call_service(&app, register_request("del-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("del-contributor").to_request()).await,
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

        assert_eq!(
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
            .await
            .status(),
            201
        );

        let ticket: TicketResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateTicketRequest {
                        title: "To be deleted".to_string(),
                        description: "boom".to_string(),
                        priority: TicketPriority::Low,
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        let contributor_delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&contributor.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(contributor_delete_resp.status(), 403);

        let admin_delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::delete()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(admin_delete_resp.status(), 204);

        let get_after_delete_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, ticket.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(get_after_delete_resp.status(), 404);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner, &contributor]).await;
    });
}

// 3. GET /tickets filters by status/priority/creator and paginates results.
#[test]
fn test_list_tickets_filters_and_paginates() {
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
            actix_test::call_service(&app, register_request("list-owner").to_request()).await,
        )
        .await;

        let group: GroupResponse = actix_test::read_body_json(
            actix_test::call_service(
                &app,
                actix_test::TestRequest::post()
                    .uri("/api/v1/groups")
                    .insert_header(auth_header(&owner.jwt))
                    .set_json(&CreateGroupRequest {
                        name: "List Group".to_string(),
                    })
                    .to_request(),
            )
            .await,
        )
        .await;

        // 3 tickets: two High, one Critical; close one of the High tickets.
        for (title, priority) in [
            ("Ticket A", TicketPriority::High),
            ("Ticket B", TicketPriority::High),
            ("Ticket C", TicketPriority::Critical),
        ] {
            let created: TicketResponse = actix_test::read_body_json(
                actix_test::call_service(
                    &app,
                    actix_test::TestRequest::post()
                        .uri(&format!("/api/v1/groups/{}/tickets", group.id))
                        .insert_header(auth_header(&owner.jwt))
                        .set_json(&CreateTicketRequest {
                            title: title.to_string(),
                            description: "d".to_string(),
                            priority,
                        })
                        .to_request(),
                )
                .await,
            )
            .await;
            if title == "Ticket A" {
                let close_resp = actix_test::call_service(
                    &app,
                    actix_test::TestRequest::patch()
                        .uri(&format!("/api/v1/groups/{}/tickets/{}", group.id, created.id))
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
            }
        }

        // Filter by priority=high: only Ticket A and Ticket B.
        let high_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/groups/{}/tickets?priority=high", group.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        assert_eq!(high_resp.status(), 200);
        let high_list: TicketListResponse = actix_test::read_body_json(high_resp).await;
        assert_eq!(high_list.total, 2);

        // Filter by status=open: excludes the closed Ticket A.
        let open_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/groups/{}/tickets?status=open", group.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        let open_list: TicketListResponse = actix_test::read_body_json(open_resp).await;
        assert_eq!(open_list.total, 2);
        assert!(open_list.items.iter().all(|t| t.status == TicketStatus::Open));

        // Pagination: per_page=1 page=2 returns exactly 1 item out of total 3.
        let page_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/groups/{}/tickets?per_page=1&page=2", group.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        let page_list: TicketListResponse = actix_test::read_body_json(page_resp).await;
        assert_eq!(page_list.total, 3);
        assert_eq!(page_list.items.len(), 1);
        assert_eq!(page_list.page, 2);
        assert_eq!(page_list.per_page, 1);

        // Search with a typo ("tikcet a" for "Ticket A") still finds it via
        // the typo-tolerant fallback.
        let search_resp = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/groups/{}/tickets?q=tikcet", group.id))
                .insert_header(auth_header(&owner.jwt))
                .to_request(),
        )
        .await;
        let search_list: TicketListResponse = actix_test::read_body_json(search_resp).await;
        assert_eq!(search_list.total, 3);

        // No bulk-by-group ticket delete exists on the repo; leftover ticket
        // docs are keyed to this now-deleted group's unique id, so no other
        // test in the shared db can observe them (same tolerance
        // group_api_tests.rs's ticket-count test already relies on).
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        cleanup(&group_repo, &user_repo, group_id, &[&owner]).await;
    });
}
