use actix_web::{App, test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::config::Config;
use resolve::group::models::{
    AddMemberRequest, CreateGroupRequest, GroupResponse, MemberResponse, Role, UpdateMemberRoleRequest,
    UserLookupResponse,
};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

// Doesn't drop collections — shared "resolve_test" db with other test files,
// same convention as tests/api_tests.rs's setup_db(). Each test uses unique,
// randomly-suffixed emails so it never collides with leftover data from a
// previous failed/panicked run.
async fn setup_db() -> (Database, String) {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("failed to connect to MongoDB");
    let db = client.database("resolve_test");

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
    test::TestRequest::post()
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

// 1. Create a group, then list it back for the creator.
#[actix_web::test]
async fn test_create_and_list_groups() {
    let (db, uri) = setup_db().await;
    let group_repo = GroupRepository::new(&db);
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let register_resp = test::call_service(&app, register_request("create-list").to_request()).await;
    assert_eq!(register_resp.status(), 201);
    let registered: AuthResponse = test::read_body_json(register_resp).await;
    let (user_id, jwt) = (registered.user.id, registered.jwt);

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&jwt))
        .set_json(&CreateGroupRequest {
            name: "Acme".to_string(),
        })
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), 201);
    let group: GroupResponse = test::read_body_json(create_resp).await;
    assert_eq!(group.name, "Acme");

    let list_req = test::TestRequest::get()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&jwt))
        .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), 200);
    let groups: Vec<GroupResponse> = test::read_body_json(list_resp).await;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, group.id);

    let group_id = ObjectId::parse_str(&group.id).unwrap();
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    user_repo.delete(ObjectId::parse_str(&user_id).unwrap()).await.ok();
}

// 2. POST /groups with no Authorization header is rejected.
#[actix_web::test]
async fn test_create_group_requires_auth() {
    let (db, uri) = setup_db().await;
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .set_json(&CreateGroupRequest {
            name: "NoAuth".to_string(),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

// 3. A non-member gets 403 fetching a group.
#[actix_web::test]
async fn test_get_group_non_member_forbidden() {
    let (db, uri) = setup_db().await;
    let group_repo = GroupRepository::new(&db);
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let owner: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("get-owner").to_request()).await).await;
    let outsider: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("get-outsider").to_request()).await).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "Private".to_string(),
        })
        .to_request();
    let group: GroupResponse = test::read_body_json(test::call_service(&app, create_req).await).await;

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/groups/{}", group.id))
        .insert_header(auth_header(&outsider.jwt))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 403);

    let group_id = ObjectId::parse_str(&group.id).unwrap();
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    user_repo.delete(ObjectId::parse_str(&owner.user.id).unwrap()).await.ok();
    user_repo.delete(ObjectId::parse_str(&outsider.user.id).unwrap()).await.ok();
}

// 4. A Contributor is forbidden from adding members.
#[actix_web::test]
async fn test_add_member_forbidden_for_contributor() {
    let (db, uri) = setup_db().await;
    let group_repo = GroupRepository::new(&db);
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let owner: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("add-owner").to_request()).await).await;
    let contributor: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("add-contributor").to_request()).await)
            .await;
    let outsider: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("add-outsider").to_request()).await).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "Team".to_string(),
        })
        .to_request();
    let group: GroupResponse = test::read_body_json(test::call_service(&app, create_req).await).await;

    let add_contributor_req = test::TestRequest::post()
        .uri(&format!("/api/v1/groups/{}/users", group.id))
        .insert_header(auth_header(&owner.jwt))
        .set_json(&AddMemberRequest {
            user_id: contributor.user.id.clone(),
            role: Role::Contributor,
        })
        .to_request();
    let add_contributor_resp = test::call_service(&app, add_contributor_req).await;
    assert_eq!(add_contributor_resp.status(), 201);

    let forbidden_req = test::TestRequest::post()
        .uri(&format!("/api/v1/groups/{}/users", group.id))
        .insert_header(auth_header(&contributor.jwt))
        .set_json(&AddMemberRequest {
            user_id: outsider.user.id.clone(),
            role: Role::Contributor,
        })
        .to_request();
    let forbidden_resp = test::call_service(&app, forbidden_req).await;
    assert_eq!(forbidden_resp.status(), 403);

    let group_id = ObjectId::parse_str(&group.id).unwrap();
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    for user in [owner, contributor, outsider] {
        user_repo.delete(ObjectId::parse_str(&user.user.id).unwrap()).await.ok();
    }
}

// 5. Promote a Contributor to Group Admin, then the original admin can leave
// (two admins now) — the sole-admin guard only kicks in below two.
#[actix_web::test]
async fn test_promote_then_original_admin_can_leave() {
    let (db, uri) = setup_db().await;
    let group_repo = GroupRepository::new(&db);
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let owner: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("promote-owner").to_request()).await).await;
    let member: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("promote-member").to_request()).await).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "Succession".to_string(),
        })
        .to_request();
    let group: GroupResponse = test::read_body_json(test::call_service(&app, create_req).await).await;

    let add_req = test::TestRequest::post()
        .uri(&format!("/api/v1/groups/{}/users", group.id))
        .insert_header(auth_header(&owner.jwt))
        .set_json(&AddMemberRequest {
            user_id: member.user.id.clone(),
            role: Role::Contributor,
        })
        .to_request();
    assert_eq!(test::call_service(&app, add_req).await.status(), 201);

    // Blocked while still the sole Group Admin.
    let leave_too_early_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/groups/{}/users/{}", group.id, owner.user.id))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, leave_too_early_req).await.status(), 409);

    let promote_req = test::TestRequest::patch()
        .uri(&format!("/api/v1/groups/{}/users/{}", group.id, member.user.id))
        .insert_header(auth_header(&owner.jwt))
        .set_json(&UpdateMemberRoleRequest {
            role: Role::GroupAdmin,
        })
        .to_request();
    let promote_resp = test::call_service(&app, promote_req).await;
    assert_eq!(promote_resp.status(), 200);
    let promoted: MemberResponse = test::read_body_json(promote_resp).await;
    assert_eq!(promoted.role, Role::GroupAdmin);

    // Now succeeds, since there are two admins.
    let leave_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/groups/{}/users/{}", group.id, owner.user.id))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, leave_req).await.status(), 204);

    let group_id = ObjectId::parse_str(&group.id).unwrap();
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    user_repo.delete(ObjectId::parse_str(&owner.user.id).unwrap()).await.ok();
    user_repo.delete(ObjectId::parse_str(&member.user.id).unwrap()).await.ok();
}

// 6. Rename then delete a group; afterward, the former owner gets 403 (no
// membership row left to prove otherwise).
#[actix_web::test]
async fn test_rename_and_delete_group() {
    let (db, uri) = setup_db().await;
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let owner: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("rename-owner").to_request()).await).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "Old Name".to_string(),
        })
        .to_request();
    let group: GroupResponse = test::read_body_json(test::call_service(&app, create_req).await).await;

    let rename_req = test::TestRequest::patch()
        .uri(&format!("/api/v1/groups/{}", group.id))
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "New Name".to_string(),
        })
        .to_request();
    let rename_resp = test::call_service(&app, rename_req).await;
    assert_eq!(rename_resp.status(), 200);
    let renamed: GroupResponse = test::read_body_json(rename_resp).await;
    assert_eq!(renamed.name, "New Name");

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/groups/{}", group.id))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, delete_req).await.status(), 204);

    let get_after_delete_req = test::TestRequest::get()
        .uri(&format!("/api/v1/groups/{}", group.id))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, get_after_delete_req).await.status(), 403);

    user_repo.delete(ObjectId::parse_str(&owner.user.id).unwrap()).await.ok();
}

// 7. Group Admin looks up a user by exact email to get their user_id; a
// Contributor is forbidden from the same lookup; a non-matching email 404s.
#[actix_web::test]
async fn test_lookup_user_by_email() {
    let (db, uri) = setup_db().await;
    let group_repo = GroupRepository::new(&db);
    let user_repo = UserRepository::new(&db);
    let app = test::init_service(
        App::new()
            .app_data(build_app_state(db, uri))
            .service(web::scope("/api/v1").configure(routes::configure)),
    )
    .await;

    let owner: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("lookup-owner").to_request()).await).await;
    let contributor: AuthResponse = test::read_body_json(
        test::call_service(&app, register_request("lookup-contributor").to_request()).await,
    )
    .await;
    let target: AuthResponse =
        test::read_body_json(test::call_service(&app, register_request("lookup-target").to_request()).await).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/groups")
        .insert_header(auth_header(&owner.jwt))
        .set_json(&CreateGroupRequest {
            name: "Team".to_string(),
        })
        .to_request();
    let group: GroupResponse = test::read_body_json(test::call_service(&app, create_req).await).await;

    let add_contributor_req = test::TestRequest::post()
        .uri(&format!("/api/v1/groups/{}/users", group.id))
        .insert_header(auth_header(&owner.jwt))
        .set_json(&AddMemberRequest {
            user_id: contributor.user.id.clone(),
            role: Role::Contributor,
        })
        .to_request();
    assert_eq!(test::call_service(&app, add_contributor_req).await.status(), 201);

    // Owner (Group Admin) finds the exact match.
    let found_req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/groups/{}/users/lookup?email={}",
            group.id, target.user.email
        ))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    let found_resp = test::call_service(&app, found_req).await;
    assert_eq!(found_resp.status(), 200);
    let found: UserLookupResponse = test::read_body_json(found_resp).await;
    assert_eq!(found.id, target.user.id);
    assert_eq!(found.email, target.user.email);

    // Contributor is forbidden from the same lookup.
    let forbidden_req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/groups/{}/users/lookup?email={}",
            group.id, target.user.email
        ))
        .insert_header(auth_header(&contributor.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, forbidden_req).await.status(), 403);

    // No match for a nonexistent email.
    let missing_req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/groups/{}/users/lookup?email={}",
            group.id,
            unique_email("lookup-missing")
        ))
        .insert_header(auth_header(&owner.jwt))
        .to_request();
    assert_eq!(test::call_service(&app, missing_req).await.status(), 404);

    let group_id = ObjectId::parse_str(&group.id).unwrap();
    group_repo.delete_members_by_group(group_id).await.ok();
    group_repo.delete_group(group_id).await.ok();
    for user in [owner, contributor, target] {
        user_repo.delete(ObjectId::parse_str(&user.user.id).unwrap()).await.ok();
    }
}
