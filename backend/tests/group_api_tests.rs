use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::config::Config;
use resolve::group::models::{
    AddMemberRequest, CreateGroupRequest, GroupResponse, MemberResponse, Role,
    UpdateMemberRoleRequest, UserLookupResponse,
};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

// Doesn't drop collections — shared "resolve_test" db with other test files,
// same convention as tests/api_tests.rs's setup_db(). Each test uses unique,
// randomly-suffixed emails so it never collides with leftover data from a
// previous failed/panicked run.
async fn setup_db() -> (Database, String) {
    let db = support::shared_client().await.database("resolve_test");
    // Only needed to populate Config below (never used to open a connection
    // at request time — see src/config.rs); cheap env lookup, no network.
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

// 1. Create a group, then list it back for the creator.
#[test]
fn test_create_and_list_groups() {
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

        let register_resp =
            actix_test::call_service(&app, register_request("create-list").to_request()).await;
        assert_eq!(register_resp.status(), 201);
        let registered: AuthResponse = actix_test::read_body_json(register_resp).await;
        let (user_id, jwt) = (registered.user.id, registered.jwt);

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&jwt))
            .set_json(&CreateGroupRequest {
                name: "Acme".to_string(),
            })
            .to_request();
        let create_resp = actix_test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), 201);
        let group: GroupResponse = actix_test::read_body_json(create_resp).await;
        assert_eq!(group.name, "Acme");

        let list_req = actix_test::TestRequest::get()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&jwt))
            .to_request();
        let list_resp = actix_test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), 200);
        let groups: Vec<GroupResponse> = actix_test::read_body_json(list_resp).await;
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, group.id);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        user_repo
            .delete(ObjectId::parse_str(&user_id).unwrap())
            .await
            .ok();
    });
}

// 2. POST /groups with no Authorization header is rejected.
#[test]
fn test_create_group_requires_auth() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .set_json(&CreateGroupRequest {
                name: "NoAuth".to_string(),
            })
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    });
}

// 3. A non-member gets 403 fetching a group.
#[test]
fn test_get_group_non_member_forbidden() {
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
            actix_test::call_service(&app, register_request("get-owner").to_request()).await,
        )
        .await;
        let outsider: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("get-outsider").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "Private".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let get_req = actix_test::TestRequest::get()
            .uri(&format!("/api/v1/groups/{}", group.id))
            .insert_header(auth_header(&outsider.jwt))
            .to_request();
        let get_resp = actix_test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), 403);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        user_repo
            .delete(ObjectId::parse_str(&owner.user.id).unwrap())
            .await
            .ok();
        user_repo
            .delete(ObjectId::parse_str(&outsider.user.id).unwrap())
            .await
            .ok();
    });
}

// 4. A Contributor is forbidden from adding members.
#[test]
fn test_add_member_forbidden_for_contributor() {
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
            actix_test::call_service(&app, register_request("add-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("add-contributor").to_request()).await,
        )
        .await;
        let outsider: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("add-outsider").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "Team".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let add_contributor_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: contributor.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        let add_contributor_resp = actix_test::call_service(&app, add_contributor_req).await;
        assert_eq!(add_contributor_resp.status(), 201);

        let forbidden_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&contributor.jwt))
            .set_json(&AddMemberRequest {
                user_id: outsider.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        let forbidden_resp = actix_test::call_service(&app, forbidden_req).await;
        assert_eq!(forbidden_resp.status(), 403);

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        for user in [owner, contributor, outsider] {
            user_repo
                .delete(ObjectId::parse_str(&user.user.id).unwrap())
                .await
                .ok();
        }
    });
}

// 5. Promote a Contributor to Group Admin, then the original admin can leave
// (two admins now) — the sole-admin guard only kicks in below two.
#[test]
fn test_promote_then_original_admin_can_leave() {
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
            actix_test::call_service(&app, register_request("promote-owner").to_request()).await,
        )
        .await;
        let member: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("promote-member").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "Succession".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let add_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: member.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        assert_eq!(actix_test::call_service(&app, add_req).await.status(), 201);

        // Blocked while still the sole Group Admin.
        let leave_too_early_req = actix_test::TestRequest::delete()
            .uri(&format!(
                "/api/v1/groups/{}/users/{}",
                group.id, owner.user.id
            ))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, leave_too_early_req)
                .await
                .status(),
            409
        );

        let promote_req = actix_test::TestRequest::patch()
            .uri(&format!(
                "/api/v1/groups/{}/users/{}",
                group.id, member.user.id
            ))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&UpdateMemberRoleRequest {
                role: Role::GroupAdmin,
            })
            .to_request();
        let promote_resp = actix_test::call_service(&app, promote_req).await;
        assert_eq!(promote_resp.status(), 200);
        let promoted: MemberResponse = actix_test::read_body_json(promote_resp).await;
        assert_eq!(promoted.role, Role::GroupAdmin);

        // Now succeeds, since there are two admins.
        let leave_req = actix_test::TestRequest::delete()
            .uri(&format!(
                "/api/v1/groups/{}/users/{}",
                group.id, owner.user.id
            ))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, leave_req).await.status(),
            204
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        user_repo
            .delete(ObjectId::parse_str(&owner.user.id).unwrap())
            .await
            .ok();
        user_repo
            .delete(ObjectId::parse_str(&member.user.id).unwrap())
            .await
            .ok();
    });
}

// 6. Rename then delete a group; afterward, the former owner gets 403 (no
// membership row left to prove otherwise).
#[test]
fn test_rename_and_delete_group() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("rename-owner").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "Old Name".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let rename_req = actix_test::TestRequest::patch()
            .uri(&format!("/api/v1/groups/{}", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "New Name".to_string(),
            })
            .to_request();
        let rename_resp = actix_test::call_service(&app, rename_req).await;
        assert_eq!(rename_resp.status(), 200);
        let renamed: GroupResponse = actix_test::read_body_json(rename_resp).await;
        assert_eq!(renamed.name, "New Name");

        let delete_req = actix_test::TestRequest::delete()
            .uri(&format!("/api/v1/groups/{}", group.id))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            204
        );

        let get_after_delete_req = actix_test::TestRequest::get()
            .uri(&format!("/api/v1/groups/{}", group.id))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, get_after_delete_req)
                .await
                .status(),
            403
        );

        user_repo
            .delete(ObjectId::parse_str(&owner.user.id).unwrap())
            .await
            .ok();
    });
}

// 7. Group Admin looks up a user by exact email to get their user_id; a
// Contributor is forbidden from the same lookup; a non-matching email 404s.
#[test]
fn test_lookup_user_by_email() {
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
            actix_test::call_service(&app, register_request("lookup-owner").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("lookup-contributor").to_request())
                .await,
        )
        .await;
        let target: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("lookup-target").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "Team".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let add_contributor_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: contributor.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, add_contributor_req)
                .await
                .status(),
            201
        );

        // Owner (Group Admin) finds the exact match.
        let found_req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/groups/{}/users/lookup?email={}",
                group.id, target.user.email
            ))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        let found_resp = actix_test::call_service(&app, found_req).await;
        assert_eq!(found_resp.status(), 200);
        let found: UserLookupResponse = actix_test::read_body_json(found_resp).await;
        assert_eq!(found.id, target.user.id);
        assert_eq!(found.email, target.user.email);

        // Contributor is forbidden from the same lookup.
        let forbidden_req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/groups/{}/users/lookup?email={}",
                group.id, target.user.email
            ))
            .insert_header(auth_header(&contributor.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, forbidden_req).await.status(),
            403
        );

        // No match for a nonexistent email.
        let missing_req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/groups/{}/users/lookup?email={}",
                group.id,
                unique_email("lookup-missing")
            ))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, missing_req).await.status(),
            404
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        for user in [owner, contributor, target] {
            user_repo
                .delete(ObjectId::parse_str(&user.user.id).unwrap())
                .await
                .ok();
        }
    });
}
