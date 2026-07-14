use std::collections::HashMap;

use mongodb::{
    Database, IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use resolve::admin::{repository::AdminRepository, service::AdminService};
use resolve::errors::ApiError;
use resolve::group::{models::Role, repository::GroupRepository, service::GroupService};
use resolve::user::models::{CreateUserInput, GlobalRole};
use resolve::user::repository::UserRepository;

mod support;

async fn setup() -> (
    Database,
    AdminService,
    GroupService,
    UserRepository,
    AdminRepository,
) {
    let db = support::shared_client().await.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    db.collection::<mongodb::bson::Document>("users")
        .drop()
        .await
        .expect("failed to drop users collection");
    db.collection::<mongodb::bson::Document>("groups")
        .drop()
        .await
        .expect("failed to drop groups collection");
    db.collection::<mongodb::bson::Document>("group_members")
        .drop()
        .await
        .expect("failed to drop group_members collection");
    db.collection::<mongodb::bson::Document>("admin_audit_log")
        .drop()
        .await
        .expect("failed to drop admin_audit_log collection");

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
        AdminService::new(&db),
        GroupService::new(&db),
        UserRepository::new(&db),
        AdminRepository::new(&db),
    )
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
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

// No repository method exists to promote a user's global_role (not needed
// anywhere in production code yet), so this reaches into the collection
// directly — acceptable for test setup, matching how other test files create
// indexes directly rather than through a repository.
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

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
}

fn assert_conflict<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Conflict(_))), "{result:?}");
}

// 1. A non-System-Admin caller is forbidden from running deletion_check.
#[test]
fn test_deletion_check_requires_system_admin() {
    support::runtime().block_on(async {
        let (_db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "not-admin").await;
        let target_id = create_user(&users, "target").await;

        let result = admin.deletion_check(caller_id, target_id).await;
        assert_forbidden(result);
    });
}

// 2. A user with no group memberships blocks nothing.
#[test]
fn test_deletion_check_no_memberships() {
    support::runtime().block_on(async {
        let (db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "lonely").await;

        let check = admin
            .deletion_check(caller_id, target_id)
            .await
            .expect("deletion_check failed");
        assert!(check.blocked_groups.is_empty());
        assert!(check.auto_delete_groups.is_empty());
    });
}

// 3. Sole Group Admin with other members shows up as blocked, with the other
// member listed as an eligible successor.
#[test]
fn test_deletion_check_blocked_group_with_successor() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "sole-admin").await;
        let contributor_id = create_user(&users, "contributor").await;

        let group = groups
            .create_group(target_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(target_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add member failed");

        let check = admin
            .deletion_check(caller_id, target_id)
            .await
            .expect("deletion_check failed");

        assert_eq!(check.blocked_groups.len(), 1);
        assert!(check.auto_delete_groups.is_empty());
        assert_eq!(check.blocked_groups[0].group_id, group.id);
        assert_eq!(check.blocked_groups[0].eligible_successors.len(), 1);
        assert_eq!(
            check.blocked_groups[0].eligible_successors[0].user_id,
            contributor_id.to_hex()
        );
    });
}

// 4. Sole Group Admin with no other members is flagged for auto-deletion.
#[test]
fn test_deletion_check_auto_delete_group() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "lone-admin").await;

        let group = groups
            .create_group(target_id, "SoloTeam".to_string())
            .await
            .expect("create group failed");

        let check = admin
            .deletion_check(caller_id, target_id)
            .await
            .expect("deletion_check failed");

        assert!(check.blocked_groups.is_empty());
        assert_eq!(check.auto_delete_groups.len(), 1);
        assert_eq!(check.auto_delete_groups[0].group_id, group.id);
    });
}

// 5. deletion_check for a nonexistent target returns NotFound.
#[test]
fn test_deletion_check_target_not_found() {
    support::runtime().block_on(async {
        let (db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;

        let result = admin.deletion_check(caller_id, ObjectId::new()).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    });
}

// 6. delete_user without a required successor is rejected, and nothing is mutated.
#[test]
fn test_delete_user_missing_successor_conflict() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "sole-admin").await;
        let contributor_id = create_user(&users, "contributor").await;

        let group = groups
            .create_group(target_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(target_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add member failed");

        let result = admin
            .delete_user(caller_id, target_id, HashMap::new())
            .await;
        assert_conflict(result);

        // Nothing was mutated: target user still exists and is still the admin.
        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_some()
        );
        let members = groups
            .list_members(target_id, group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 2);
    });
}

// 7. delete_user with a valid successor promotes them, removes the deleted
// user's membership, deletes the user, and writes one audit entry.
#[test]
fn test_delete_user_with_successor_succeeds() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "sole-admin").await;
        let contributor_id = create_user(&users, "contributor").await;

        let group = groups
            .create_group(target_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(target_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add member failed");

        let mut successors = HashMap::new();
        successors.insert(group_id, contributor_id);
        admin
            .delete_user(caller_id, target_id, successors)
            .await
            .expect("delete_user failed");

        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_none()
        );

        let members = groups
            .list_members(contributor_id, group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, contributor_id.to_hex());
        assert_eq!(members[0].role, Role::GroupAdmin);

        let entries = audit
            .list_audit_log_for_group(group_id)
            .await
            .expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].successor_user_id, Some(contributor_id));
    });
}

// 8. delete_user for a sole-member group deletes the group outright and
// writes a group_auto_deleted audit entry.
#[test]
fn test_delete_user_auto_deletes_lone_group() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "lone-admin").await;

        let group = groups
            .create_group(target_id, "SoloTeam".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        admin
            .delete_user(caller_id, target_id, HashMap::new())
            .await
            .expect("delete_user failed");

        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_none()
        );

        let entries = audit
            .list_audit_log_for_group(group_id)
            .await
            .expect("list failed");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].successor_user_id.is_none());
    });
}

// 9. delete_user for a plain Contributor membership just removes it — no
// successor needed, no audit entry.
#[test]
fn test_delete_user_removes_plain_membership() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let owner_id = create_user(&users, "owner").await;
        let target_id = create_user(&users, "contributor").await;

        let group = groups
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(owner_id, group_id, target_id, Role::Contributor)
            .await
            .expect("add member failed");

        admin
            .delete_user(caller_id, target_id, HashMap::new())
            .await
            .expect("delete_user failed");

        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_none()
        );
        let members = groups
            .list_members(owner_id, group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 1);

        let entries = audit
            .list_audit_log_for_user(target_id)
            .await
            .expect("list failed");
        assert!(entries.is_empty());
    });
}

// 10. A non-System-Admin caller cannot delete_user; nothing is mutated.
#[test]
fn test_delete_user_requires_system_admin() {
    support::runtime().block_on(async {
        let (_db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "not-admin").await;
        let target_id = create_user(&users, "target").await;

        let result = admin
            .delete_user(caller_id, target_id, HashMap::new())
            .await;
        assert_forbidden(result);
        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_some()
        );
    });
}

// 11. Sole admin of two groups but only one successor supplied — the whole
// deletion is rejected, and *neither* group is mutated (validated up front).
#[test]
fn test_delete_user_partial_successors_rejected_atomically() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let target_id = create_user(&users, "sole-admin").await;
        let contributor_a = create_user(&users, "contributor-a").await;
        let contributor_b = create_user(&users, "contributor-b").await;

        let group_a = groups
            .create_group(target_id, "TeamA".to_string())
            .await
            .expect("create group failed");
        let group_a_id = ObjectId::parse_str(&group_a.id).unwrap();
        groups
            .add_member(target_id, group_a_id, contributor_a, Role::Contributor)
            .await
            .expect("add member failed");

        let group_b = groups
            .create_group(target_id, "TeamB".to_string())
            .await
            .expect("create group failed");
        let group_b_id = ObjectId::parse_str(&group_b.id).unwrap();
        groups
            .add_member(target_id, group_b_id, contributor_b, Role::Contributor)
            .await
            .expect("add member failed");

        let mut successors = HashMap::new();
        successors.insert(group_a_id, contributor_a); // group_b's successor is missing

        let result = admin.delete_user(caller_id, target_id, successors).await;
        assert_conflict(result);

        assert!(
            users
                .find_by_id(target_id)
                .await
                .expect("find failed")
                .is_some()
        );
        let members_a = groups
            .list_members(target_id, group_a_id)
            .await
            .expect("list failed");
        assert_eq!(members_a.len(), 2); // group A untouched despite having a valid successor supplied
        let members_b = groups
            .list_members(target_id, group_b_id)
            .await
            .expect("list failed");
        assert_eq!(members_b.len(), 2);
    });
}

// 12. list_users requires System Admin.
#[test]
fn test_list_users_requires_system_admin() {
    support::runtime().block_on(async {
        let (_db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "not-admin").await;

        let result = admin.list_users(caller_id).await;
        assert_forbidden(result);
    });
}

// 13. list_users returns every user.
#[test]
fn test_list_users_returns_all_users() {
    support::runtime().block_on(async {
        let (db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        create_user(&users, "someone").await;
        create_user(&users, "someone-else").await;

        let all_users = admin
            .list_users(caller_id)
            .await
            .expect("list_users failed");
        assert_eq!(all_users.len(), 3); // caller + the two created above
    });
}

// 14. list_groups requires System Admin.
#[test]
fn test_list_groups_requires_system_admin() {
    support::runtime().block_on(async {
        let (_db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "not-admin").await;

        let result = admin.list_groups(caller_id).await;
        assert_forbidden(result);
    });
}

// 15. list_groups returns every group, regardless of membership.
#[test]
fn test_list_groups_returns_all_groups() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let owner_id = create_user(&users, "owner").await;

        groups
            .create_group(owner_id, "Team A".to_string())
            .await
            .expect("create group failed");
        groups
            .create_group(owner_id, "Team B".to_string())
            .await
            .expect("create group failed");

        let all_groups = admin
            .list_groups(caller_id)
            .await
            .expect("list_groups failed");
        assert_eq!(all_groups.len(), 2);
    });
}

// 16. delete_group requires System Admin.
#[test]
fn test_delete_group_requires_system_admin() {
    support::runtime().block_on(async {
        let (_db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "not-admin").await;
        let owner_id = create_user(&users, "owner").await;
        let group = groups
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = admin.delete_group(caller_id, group_id).await;
        assert_forbidden(result);
    });
}

// 17. System Admin can delete a group they are not a member of — no
// membership check, no successor needed, cascades its members.
#[test]
fn test_delete_group_as_non_member_succeeds() {
    support::runtime().block_on(async {
        let (db, admin, groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;
        let owner_id = create_user(&users, "owner").await;
        let contributor_id = create_user(&users, "contributor").await;

        let group = groups
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create group failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();
        groups
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add member failed");

        admin
            .delete_group(caller_id, group_id)
            .await
            .expect("delete_group failed");

        let group_repo = GroupRepository::new(&db);
        assert!(
            group_repo
                .find_group_by_id(group_id)
                .await
                .expect("find failed")
                .is_none()
        );
        let members = group_repo
            .list_members(group_id)
            .await
            .expect("list failed");
        assert!(members.is_empty());
    });
}

// 18. delete_group for a nonexistent group returns NotFound.
#[test]
fn test_delete_group_not_found() {
    support::runtime().block_on(async {
        let (db, admin, _groups, users, _audit) = setup().await;
        let caller_id = create_user(&users, "sysadmin").await;
        make_system_admin(&db, caller_id).await;

        let result = admin.delete_group(caller_id, ObjectId::new()).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    });
}
