use actix_web::{App, HttpResponse, test as actix_test, web};
use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::auth::jwt;
use resolve::config::Config;
use resolve::group::models::Role;
use resolve::group::repository::GroupRepository;
use resolve::server::middleware::{GroupScoped, SystemAdminUser};
use resolve::state::AppState;
use resolve::user::models::{CreateUserInput, GlobalRole};
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

// Test-only routes that do nothing but exercise the extractors: reaching the
// handler at all proves extraction succeeded. group_probe echoes back the role
// the extractor resolved so tests can assert it.
async fn group_probe(scoped: GroupScoped) -> HttpResponse {
    HttpResponse::Ok().json(scoped.role)
}

async fn admin_probe(_admin: SystemAdminUser) -> HttpResponse {
    HttpResponse::Ok().finish()
}

// Doesn't drop collections — shared "resolve_test" db with other test files.
// Group/user ids are freshly generated per test, so rows never collide with
// leftovers from other runs.
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

macro_rules! test_app {
    ($state:expr) => {
        actix_test::init_service(
            App::new()
                .app_data($state)
                .service(web::resource("/groups/{id}/probe").route(web::get().to(group_probe)))
                .service(web::resource("/admin/probe").route(web::get().to(admin_probe))),
        )
        .await
    };
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
}

fn auth_header(user_id: ObjectId) -> (&'static str, String) {
    let token = jwt::issue_token(&user_id.to_hex(), TEST_JWT_SECRET).expect("issue token failed");
    ("Authorization", format!("Bearer {token}"))
}

async fn seed_user(user_repo: &UserRepository, prefix: &str) -> ObjectId {
    let user = user_repo
        .create(CreateUserInput {
            email: unique_email(prefix),
            name: prefix.to_string(),
            password_hash: "hash".to_string(),
        })
        .await
        .expect("create user failed");
    user.id.unwrap()
}

async fn make_system_admin(db: &Database, user_id: ObjectId) {
    let role = mongodb::bson::to_bson(&GlobalRole::SystemAdmin).unwrap();
    db.collection::<mongodb::bson::Document>("users")
        .update_one(
            doc! { "_id": user_id },
            doc! { "$set": { "global_role": role } },
        )
        .await
        .expect("failed to promote to system admin");
}

// --- GroupScoped ---

// 1. A member reaches the handler; the resolved role matches their membership.
#[test]
fn test_group_scoped_member_resolves_role() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let groups = GroupRepository::new(&db);
        let app = test_app!(build_app_state(db.clone(), uri));

        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        let req = actix_test::TestRequest::get()
            .uri(&format!("/groups/{}/probe", group_id.to_hex()))
            .insert_header(auth_header(user_id))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let role: Role = actix_test::read_body_json(resp).await;
        assert_eq!(role, Role::Contributor);
    });
}

// 2. A Group Admin member resolves the GroupAdmin role.
#[test]
fn test_group_scoped_admin_resolves_role() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let groups = GroupRepository::new(&db);
        let app = test_app!(build_app_state(db.clone(), uri));

        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::GroupAdmin)
            .await
            .expect("insert failed");

        let req = actix_test::TestRequest::get()
            .uri(&format!("/groups/{}/probe", group_id.to_hex()))
            .insert_header(auth_header(user_id))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let role: Role = actix_test::read_body_json(resp).await;
        assert_eq!(role, Role::GroupAdmin);
    });
}

// 3. A non-member is forbidden.
#[test]
fn test_group_scoped_non_member_forbidden() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app = test_app!(build_app_state(db, uri));

        let req = actix_test::TestRequest::get()
            .uri(&format!("/groups/{}/probe", oid().to_hex()))
            .insert_header(auth_header(oid()))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

// 4. Membership revoked after the token was issued => next request is 403.
// This is the whole point of resolving the role per-request instead of baking
// it into the JWT.
#[test]
fn test_group_scoped_revoked_membership_forbidden() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let groups = GroupRepository::new(&db);
        let app = test_app!(build_app_state(db.clone(), uri));

        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        let uri_path = format!("/groups/{}/probe", group_id.to_hex());
        let header = auth_header(user_id);

        let ok = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&uri_path)
                .insert_header(header.clone())
                .to_request(),
        )
        .await;
        assert_eq!(ok.status(), 200);

        groups
            .delete_member(group_id, user_id)
            .await
            .expect("delete failed");

        let forbidden = actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&uri_path)
                .insert_header(header)
                .to_request(),
        )
        .await;
        assert_eq!(forbidden.status(), 403);
    });
}

// 5. No Authorization header => 401, regardless of membership.
#[test]
fn test_group_scoped_missing_auth_unauthorized() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app = test_app!(build_app_state(db, uri));

        let req = actix_test::TestRequest::get()
            .uri(&format!("/groups/{}/probe", oid().to_hex()))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    });
}

// 6. A malformed group id in the path is a client error (400), not a 403/500.
#[test]
fn test_group_scoped_invalid_group_id_bad_request() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app = test_app!(build_app_state(db, uri));

        let req = actix_test::TestRequest::get()
            .uri("/groups/not-an-oid/probe")
            .insert_header(auth_header(oid()))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    });
}

// --- SystemAdminUser ---

// 7. A System Admin reaches the handler.
#[test]
fn test_system_admin_allows_admin() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let users = UserRepository::new(&db);
        let user_id = seed_user(&users, "sysadmin").await;
        make_system_admin(&db, user_id).await;
        let app = test_app!(build_app_state(db.clone(), uri));

        let req = actix_test::TestRequest::get()
            .uri("/admin/probe")
            .insert_header(auth_header(user_id))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    });
}

// 8. A regular user is forbidden from /admin routes.
#[test]
fn test_system_admin_forbids_regular_user() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let users = UserRepository::new(&db);
        let user_id = seed_user(&users, "regular").await;
        let app = test_app!(build_app_state(db.clone(), uri));

        let req = actix_test::TestRequest::get()
            .uri("/admin/probe")
            .insert_header(auth_header(user_id))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

// 9. No Authorization header => 401.
#[test]
fn test_system_admin_missing_auth_unauthorized() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app = test_app!(build_app_state(db, uri));

        let req = actix_test::TestRequest::get()
            .uri("/admin/probe")
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    });
}
