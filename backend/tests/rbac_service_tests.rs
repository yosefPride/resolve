use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::errors::ApiError;
use resolve::group::{models::Role, repository::GroupRepository};
use resolve::rbac::service::RbacService;
use resolve::user::models::{CreateUserInput, GlobalRole};
use resolve::user::repository::UserRepository;

mod support;

async fn setup() -> (Database, RbacService, GroupRepository, UserRepository) {
    let db = support::shared_client().await.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    db.collection::<mongodb::bson::Document>("users")
        .drop()
        .await
        .expect("failed to drop users collection");
    db.collection::<mongodb::bson::Document>("group_members")
        .drop()
        .await
        .expect("failed to drop group_members collection");

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

    (
        db.clone(),
        RbacService::new(&db),
        GroupRepository::new(&db),
        UserRepository::new(&db),
    )
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
}

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
}

async fn create_user(user_repo: &UserRepository, prefix: &str) -> ObjectId {
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

// No repository method promotes a user's global_role (not needed in production
// yet), so reach into the collection directly for test setup — matching
// admin_service_tests.rs.
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

// --- require_member ---

// 1. A non-member is forbidden.
#[test]
fn test_require_member_non_member_forbidden() {
    support::runtime().block_on(async {
        let (_db, rbac, _groups, _users) = setup().await;
        let result = rbac.require_member(oid(), oid()).await;
        assert_forbidden(result);
    });
}

// 2. A Contributor member passes, and the returned membership carries its role.
#[test]
fn test_require_member_contributor_succeeds() {
    support::runtime().block_on(async {
        let (_db, rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        let member = rbac
            .require_member(group_id, user_id)
            .await
            .expect("require_member failed");
        assert_eq!(member.role, Role::Contributor);
        assert_eq!(member.user_id, user_id);
    });
}

// --- require_group_admin ---

// 3. A Contributor is forbidden from group-admin actions.
#[test]
fn test_require_group_admin_contributor_forbidden() {
    support::runtime().block_on(async {
        let (_db, rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        let result = rbac.require_group_admin(group_id, user_id).await;
        assert_forbidden(result);
    });
}

// 4. A Group Admin passes require_group_admin (and require_member).
#[test]
fn test_require_group_admin_admin_succeeds() {
    support::runtime().block_on(async {
        let (_db, rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        groups
            .insert_member(group_id, user_id, Role::GroupAdmin)
            .await
            .expect("insert failed");

        let member = rbac
            .require_group_admin(group_id, user_id)
            .await
            .expect("require_group_admin failed");
        assert_eq!(member.role, Role::GroupAdmin);

        rbac.require_member(group_id, user_id)
            .await
            .expect("admin should also pass require_member");
    });
}

// 5. A non-member is forbidden from group-admin actions too.
#[test]
fn test_require_group_admin_non_member_forbidden() {
    support::runtime().block_on(async {
        let (_db, rbac, _groups, _users) = setup().await;
        let result = rbac.require_group_admin(oid(), oid()).await;
        assert_forbidden(result);
    });
}

// --- require_system_admin ---

// 6. A regular user is forbidden.
#[test]
fn test_require_system_admin_regular_user_forbidden() {
    support::runtime().block_on(async {
        let (_db, rbac, _groups, users) = setup().await;
        let user_id = create_user(&users, "regular").await;
        let result = rbac.require_system_admin(user_id).await;
        assert_forbidden(result);
    });
}

// 7. A System Admin passes.
#[test]
fn test_require_system_admin_admin_succeeds() {
    support::runtime().block_on(async {
        let (db, rbac, _groups, users) = setup().await;
        let user_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, user_id).await;

        rbac.require_system_admin(user_id)
            .await
            .expect("system admin should pass");
    });
}

// 8. A nonexistent user is forbidden (never leaks user existence).
#[test]
fn test_require_system_admin_unknown_user_forbidden() {
    support::runtime().block_on(async {
        let (_db, rbac, _groups, _users) = setup().await;
        let result = rbac.require_system_admin(oid()).await;
        assert_forbidden(result);
    });
}

// --- require_owner_or_group_admin (pure) ---

// 9. A Contributor may act on a resource they own.
#[test]
fn test_owner_or_group_admin_contributor_owns_succeeds() {
    support::runtime().block_on(async {
        let (_db, _rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        let member = groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        RbacService::require_owner_or_group_admin(&member, user_id)
            .expect("contributor should act on own resource");
    });
}

// 10. A Contributor may not act on someone else's resource.
#[test]
fn test_owner_or_group_admin_contributor_not_owner_forbidden() {
    support::runtime().block_on(async {
        let (_db, _rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        let member = groups
            .insert_member(group_id, user_id, Role::Contributor)
            .await
            .expect("insert failed");

        let result = RbacService::require_owner_or_group_admin(&member, oid());
        assert_forbidden(result);
    });
}

// 11. A Group Admin may act on any resource in the group, regardless of owner.
#[test]
fn test_owner_or_group_admin_admin_any_resource_succeeds() {
    support::runtime().block_on(async {
        let (_db, _rbac, groups, _users) = setup().await;
        let group_id = oid();
        let user_id = oid();
        let member = groups
            .insert_member(group_id, user_id, Role::GroupAdmin)
            .await
            .expect("insert failed");

        RbacService::require_owner_or_group_admin(&member, oid())
            .expect("group admin should act on any resource");
    });
}
