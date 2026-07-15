use std::collections::HashMap;

use actix_web::{App, test as actix_test, web};
use mongodb::{Database, IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::admin::models::{AuditLogEntryResponse, DeleteUserRequest, DeletionCheckResponse};
use resolve::auth::models::{AuthResponse, RegisterRequest};
use resolve::config::Config;
use resolve::group::models::{AddMemberRequest, CreateGroupRequest, GroupResponse, Role};
use resolve::group::repository::GroupRepository;
use resolve::server::routes;
use resolve::state::AppState;
use resolve::user::models::GlobalRole;
use resolve::user::models::UserResponse;
use resolve::user::repository::UserRepository;

const TEST_JWT_SECRET: &str = "test-secret";

mod support;

// Doesn't drop collections — shared "resolve_test" db with other test files,
// same convention as tests/api_tests.rs's setup_db(). Each test uses unique,
// randomly-suffixed emails so it never collides with leftover data.
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

fn register_request(prefix: &str) -> actix_test::TestRequest {
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

// No HTTP endpoint promotes a user to System Admin (by design — see
// docs/rbac.md), so tests reach into the collection directly, same as
// tests/admin_service_tests.rs's make_system_admin helper.
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

// 1. Full flow: deletion-check shows the blocked group with its eligible
// successor, then delete commits the succession — verified both through the
// response and by checking group membership afterward.
#[test]
fn test_deletion_check_and_delete_full_flow() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sole-admin").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("contributor").to_request()).await,
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

        let add_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: contributor.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        assert_eq!(actix_test::call_service(&app, add_req).await.status(), 201);

        let check_req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/users/{}/deletion-check",
                owner.user.id
            ))
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        let check_resp = actix_test::call_service(&app, check_req).await;
        assert_eq!(check_resp.status(), 200);
        let check: DeletionCheckResponse = actix_test::read_body_json(check_resp).await;
        assert_eq!(check.blocked_groups.len(), 1);
        assert!(check.auto_delete_groups.is_empty());
        assert_eq!(check.blocked_groups[0].group_id, group.id);
        assert_eq!(
            check.blocked_groups[0].eligible_successors[0].user_id,
            contributor.user.id
        );

        let delete_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{}/delete", owner.user.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .set_json(&DeleteUserRequest {
                successors: HashMap::from([(group.id.clone(), contributor.user.id.clone())]),
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            204
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let members = group_repo
            .list_members(group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].role, Role::GroupAdmin);
        assert_eq!(
            members[0].user_id,
            ObjectId::parse_str(&contributor.user.id).unwrap()
        );

        assert!(
            user_repo
                .find_by_id(ObjectId::parse_str(&owner.user.id).unwrap())
                .await
                .expect("find failed")
                .is_none()
        );

        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
        user_repo
            .delete(ObjectId::parse_str(&contributor.user.id).unwrap())
            .await
            .ok();
    });
}

// 2. A non-System-Admin caller gets 403 from deletion-check.
#[test]
fn test_deletion_check_requires_system_admin() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let caller: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("not-admin").to_request()).await,
        )
        .await;
        let target: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("target").to_request()).await,
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/users/{}/deletion-check",
                target.user.id
            ))
            .insert_header(auth_header(&caller.jwt))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);

        user_repo
            .delete(ObjectId::parse_str(&caller.user.id).unwrap())
            .await
            .ok();
        user_repo
            .delete(ObjectId::parse_str(&target.user.id).unwrap())
            .await
            .ok();
    });
}

// 3. Deleting a sole admin without supplying a successor returns 409, and
// group membership is untouched.
#[test]
fn test_delete_user_missing_successor_returns_409() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sole-admin").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("contributor").to_request()).await,
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

        let add_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: contributor.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        assert_eq!(actix_test::call_service(&app, add_req).await.status(), 201);

        let delete_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{}/delete", owner.user.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .set_json(&DeleteUserRequest {
                successors: HashMap::new(),
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            409
        );

        assert!(
            user_repo
                .find_by_id(ObjectId::parse_str(&owner.user.id).unwrap())
                .await
                .expect("find failed")
                .is_some()
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        for id in [sysadmin.user.id, owner.user.id, contributor.user.id] {
            user_repo
                .delete(ObjectId::parse_str(&id).unwrap())
                .await
                .ok();
        }
    });
}

// 4. Deleting the sole member of a group auto-deletes the group.
#[test]
fn test_delete_user_auto_deletes_lone_group() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("lone-admin").to_request()).await,
        )
        .await;

        let create_req = actix_test::TestRequest::post()
            .uri("/api/v1/groups")
            .insert_header(auth_header(&owner.jwt))
            .set_json(&CreateGroupRequest {
                name: "SoloTeam".to_string(),
            })
            .to_request();
        let group: GroupResponse =
            actix_test::read_body_json(actix_test::call_service(&app, create_req).await).await;

        let delete_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{}/delete", owner.user.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .set_json(&DeleteUserRequest {
                successors: HashMap::new(),
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            204
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        assert!(
            group_repo
                .find_group_by_id(group_id)
                .await
                .expect("find failed")
                .is_none()
        );
        assert!(
            user_repo
                .find_by_id(ObjectId::parse_str(&owner.user.id).unwrap())
                .await
                .expect("find failed")
                .is_none()
        );

        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
    });
}

// 5. deletion-check for a nonexistent target returns 404.
#[test]
fn test_deletion_check_target_not_found() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let req = actix_test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/users/{}/deletion-check",
                ObjectId::new()
            ))
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        assert_eq!(actix_test::call_service(&app, req).await.status(), 404);

        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
    });
}

// 6. A malformed id in the path is rejected with 400 before the service layer runs.
#[test]
fn test_deletion_check_invalid_id_returns_400() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let req = actix_test::TestRequest::get()
            .uri("/api/v1/admin/users/not-an-object-id/deletion-check")
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        assert_eq!(actix_test::call_service(&app, req).await.status(), 400);

        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
    });
}

// 7. GET /admin/users and GET /admin/groups both return the expected data
// for a System Admin.
#[test]
fn test_list_users_and_list_groups() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;
        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("owner").to_request()).await,
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

        let users_req = actix_test::TestRequest::get()
            .uri("/api/v1/admin/users")
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        let users_resp = actix_test::call_service(&app, users_req).await;
        assert_eq!(users_resp.status(), 200);
        let all_users: Vec<UserResponse> = actix_test::read_body_json(users_resp).await;
        // Not an exact count: "users" is a shared, cumulative collection across
        // every HTTP-level test file (setup_db() never drops it here), so other
        // tests' data — or orphaned rows from a past run that panicked before its
        // own cleanup ran — can legitimately be present too.
        assert!(all_users.iter().any(|u| u.id == sysadmin.user.id));
        assert!(all_users.iter().any(|u| u.id == owner.user.id));

        let groups_req = actix_test::TestRequest::get()
            .uri("/api/v1/admin/groups")
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        let groups_resp = actix_test::call_service(&app, groups_req).await;
        assert_eq!(groups_resp.status(), 200);
        let all_groups: Vec<GroupResponse> = actix_test::read_body_json(groups_resp).await;
        assert!(all_groups.iter().any(|g| g.id == group.id));

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        for id in [sysadmin.user.id, owner.user.id] {
            user_repo
                .delete(ObjectId::parse_str(&id).unwrap())
                .await
                .ok();
        }
    });
}

// 8. A non-System-Admin caller gets 403 from GET /admin/users.
#[test]
fn test_list_users_requires_system_admin() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db, uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let caller: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("not-admin").to_request()).await,
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri("/api/v1/admin/users")
            .insert_header(auth_header(&caller.jwt))
            .to_request();
        assert_eq!(actix_test::call_service(&app, req).await.status(), 403);

        user_repo
            .delete(ObjectId::parse_str(&caller.user.id).unwrap())
            .await
            .ok();
    });
}

// 9. System Admin can delete a group directly, without being a member —
// membership cascades away with it.
#[test]
fn test_delete_group_as_non_member_succeeds() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;
        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("owner").to_request()).await,
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

        let delete_req = actix_test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/groups/{}", group.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            204
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        assert!(
            group_repo
                .find_group_by_id(group_id)
                .await
                .expect("find failed")
                .is_none()
        );
        assert!(
            group_repo
                .list_members(group_id)
                .await
                .expect("list failed")
                .is_empty()
        );

        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
        user_repo
            .delete(ObjectId::parse_str(&owner.user.id).unwrap())
            .await
            .ok();
    });
}

// 10. A non-System-Admin caller gets 403 from DELETE /admin/groups/:id
// (Group Admins still delete their own group via DELETE /groups/:id instead).
#[test]
fn test_delete_group_requires_system_admin() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("owner").to_request()).await,
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

        // The owner is that group's own Group Admin, but this is the *admin*
        // (System Admin) endpoint — Group Admin isn't sufficient here.
        let delete_req = actix_test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/groups/{}", group.id))
            .insert_header(auth_header(&owner.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            403
        );

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        user_repo
            .delete(ObjectId::parse_str(&owner.user.id).unwrap())
            .await
            .ok();
    });
}

// 12. After an admin-triggered succession, the new audit entry is retrievable
// via GET /admin/audit-log?group_id=... — plus the guard: 401 without a token,
// 403 for a regular (non-System-Admin) caller.
#[test]
fn test_audit_log_lists_succession_entry() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let group_repo = GroupRepository::new(&db);
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let owner: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sole-admin").to_request()).await,
        )
        .await;
        let contributor: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("contributor").to_request()).await,
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

        let add_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{}/users", group.id))
            .insert_header(auth_header(&owner.jwt))
            .set_json(&AddMemberRequest {
                user_id: contributor.user.id.clone(),
                role: Role::Contributor,
            })
            .to_request();
        assert_eq!(actix_test::call_service(&app, add_req).await.status(), 201);

        // Delete the sole admin, naming the contributor as successor — writes
        // one succession audit entry for this group.
        let delete_req = actix_test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{}/delete", owner.user.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .set_json(&DeleteUserRequest {
                successors: HashMap::from([(group.id.clone(), contributor.user.id.clone())]),
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, delete_req).await.status(),
            204
        );

        // No token → 401.
        let no_token = actix_test::TestRequest::get()
            .uri("/api/v1/admin/audit-log")
            .to_request();
        assert_eq!(actix_test::call_service(&app, no_token).await.status(), 401);

        // The promoted contributor is a regular user (Group Admin, not System
        // Admin) → 403.
        let forbidden = actix_test::TestRequest::get()
            .uri("/api/v1/admin/audit-log")
            .insert_header(auth_header(&contributor.jwt))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, forbidden).await.status(),
            403
        );

        // System Admin, filtered by this group: the succession entry is present.
        // Matched by ids (not count): admin_audit_log is a shared, cumulative
        // collection here, and other tests may have entries for other groups.
        let list_req = actix_test::TestRequest::get()
            .uri(&format!("/api/v1/admin/audit-log?group_id={}", group.id))
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        let list_resp = actix_test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), 200);
        let entries: Vec<AuditLogEntryResponse> = actix_test::read_body_json(list_resp).await;
        assert!(entries.iter().any(|e| {
            e.group_id == group.id
                && e.deleted_user_id == owner.user.id
                && e.successor_user_id.as_deref() == Some(contributor.user.id.as_str())
        }));

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        group_repo.delete_members_by_group(group_id).await.ok();
        group_repo.delete_group(group_id).await.ok();
        for id in [sysadmin.user.id, contributor.user.id] {
            user_repo
                .delete(ObjectId::parse_str(&id).unwrap())
                .await
                .ok();
        }
    });
}

// 11. DELETE /admin/groups/:id for a nonexistent group returns 404.
#[test]
fn test_delete_group_not_found() {
    support::runtime().block_on(async {
        let (db, uri) = setup_db().await;
        let user_repo = UserRepository::new(&db);
        let app = actix_test::init_service(
            App::new()
                .app_data(build_app_state(db.clone(), uri))
                .service(web::scope("/api/v1").configure(routes::configure)),
        )
        .await;

        let sysadmin: AuthResponse = actix_test::read_body_json(
            actix_test::call_service(&app, register_request("sysadmin").to_request()).await,
        )
        .await;
        make_system_admin(&db, ObjectId::parse_str(&sysadmin.user.id).unwrap()).await;

        let req = actix_test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/groups/{}", ObjectId::new()))
            .insert_header(auth_header(&sysadmin.jwt))
            .to_request();
        assert_eq!(actix_test::call_service(&app, req).await.status(), 404);

        user_repo
            .delete(ObjectId::parse_str(&sysadmin.user.id).unwrap())
            .await
            .ok();
    });
}
