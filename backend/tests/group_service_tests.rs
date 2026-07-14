use mongodb::{IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::errors::ApiError;
use resolve::group::{models::Role, service::GroupService};
use resolve::user::{models::CreateUserInput, repository::UserRepository};

mod support;

async fn setup() -> GroupService {
    let db = support::shared_client().await.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    db.collection::<mongodb::bson::Document>("groups")
        .drop()
        .await
        .expect("failed to drop groups collection");
    db.collection::<mongodb::bson::Document>("group_members")
        .drop()
        .await
        .expect("failed to drop group_members collection");

    db.collection::<mongodb::bson::Document>("group_members")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "group_id": 1, "user_id": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("failed to create group_members compound index");

    GroupService::new(&db)
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Forbidden)), "{result:?}");
}

fn assert_conflict<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::Conflict(_))), "{result:?}");
}

fn assert_not_found<T: std::fmt::Debug>(result: Result<T, ApiError>) {
    assert!(matches!(result, Err(ApiError::NotFound)), "{result:?}");
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@test.com", ObjectId::new())
}

// Seeds a real `users` document via the shared client's handle to the same
// "resolve_test" database `setup()` uses, independent of any given
// GroupService's own db handle (collections are shared by db name).
async fn seed_user(email: &str, name: &str) -> ObjectId {
    let db = support::shared_client().await.database("resolve_test");
    let user_repo = UserRepository::new(&db);
    let user = user_repo
        .create(CreateUserInput {
            email: email.to_string(),
            name: name.to_string(),
            password_hash: "irrelevant".to_string(),
        })
        .await
        .expect("failed to seed user");
    user.id.expect("insert_one always returns an id")
}

// 1. Creating a group auto-adds the creator as Group Admin.
#[test]
fn test_create_group_adds_creator_as_admin() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();

        let group = service
            .create_group(owner_id, "Acme".to_string())
            .await
            .expect("create failed");

        let members = service
            .list_members(owner_id, ObjectId::parse_str(&group.id).unwrap())
            .await
            .expect("list failed");

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].role, Role::GroupAdmin);
    });
}

// 2. list_my_groups returns only the groups the caller belongs to.
#[test]
fn test_list_my_groups_returns_only_my_groups() {
    support::runtime().block_on(async {
        let service = setup().await;
        let user_id = oid();

        service
            .create_group(user_id, "Mine".to_string())
            .await
            .expect("create failed");
        service
            .create_group(oid(), "NotMine".to_string())
            .await
            .expect("create failed");

        let groups = service.list_my_groups(user_id).await.expect("list failed");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Mine");
    });
}

// 3. A member can fetch the group.
#[test]
fn test_get_group_member_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Visible".to_string())
            .await
            .expect("create failed");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let found = service
            .get_group(owner_id, group_id)
            .await
            .expect("get failed");
        assert_eq!(found.name, "Visible");
    });
}

// 4. A non-member is forbidden from fetching the group.
#[test]
fn test_get_group_non_member_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Hidden".to_string())
            .await
            .expect("create failed");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let result = service.get_group(oid(), group_id).await;
        assert_forbidden(result);
    });
}

// 5. Group Admin can rename the group.
#[test]
fn test_rename_group_admin_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Old".to_string())
            .await
            .expect("create failed");

        let group_id = ObjectId::parse_str(&group.id).unwrap();
        let renamed = service
            .rename_group(owner_id, group_id, "New".to_string())
            .await
            .expect("rename failed");
        assert_eq!(renamed.name, "New");
    });
}

// 6. A Contributor cannot rename the group.
#[test]
fn test_rename_group_contributor_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let contributor_id = oid();
        let group = service
            .create_group(owner_id, "Old".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add failed");

        let result = service
            .rename_group(contributor_id, group_id, "New".to_string())
            .await;
        assert_forbidden(result);
    });
}

// 7. Deleting a group cascades its members.
#[test]
fn test_delete_group_cascades_members() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "ToDelete".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .delete_group(owner_id, group_id)
            .await
            .expect("delete failed");

        let result = service.get_group(owner_id, group_id).await;
        assert_forbidden(result); // no membership row left, so lookup is Forbidden
    });
}

// 8. A Contributor cannot delete the group.
#[test]
fn test_delete_group_contributor_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let contributor_id = oid();
        let group = service
            .create_group(owner_id, "Protected".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add failed");

        let result = service.delete_group(contributor_id, group_id).await;
        assert_forbidden(result);
    });
}

// 9. Group Admin can add a member.
#[test]
fn test_add_member_admin_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let new_member_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let member = service
            .add_member(owner_id, group_id, new_member_id, Role::Contributor)
            .await
            .expect("add failed");
        assert_eq!(member.role, Role::Contributor);
    });
}

// 10. A Contributor cannot add a member.
#[test]
fn test_add_member_contributor_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let contributor_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add failed");

        let result = service
            .add_member(contributor_id, group_id, oid(), Role::Contributor)
            .await;
        assert_forbidden(result);
    });
}

// 11. Adding a user who's already a member returns Conflict.
#[test]
fn test_add_member_duplicate_conflict() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let target_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, target_id, Role::Contributor)
            .await
            .expect("first add failed");

        let result = service
            .add_member(owner_id, group_id, target_id, Role::Contributor)
            .await;
        assert_conflict(result);
    });
}

// 12. Promoting a Contributor to Group Admin succeeds.
#[test]
fn test_update_member_role_promote_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let target_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, target_id, Role::Contributor)
            .await
            .expect("add failed");

        let updated = service
            .update_member_role(owner_id, group_id, target_id, Role::GroupAdmin)
            .await
            .expect("update failed");
        assert_eq!(updated.role, Role::GroupAdmin);
    });
}

// 13. Demoting the sole Group Admin is blocked.
#[test]
fn test_update_member_role_demote_sole_admin_conflict() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = service
            .update_member_role(owner_id, group_id, owner_id, Role::Contributor)
            .await;
        assert_conflict(result);
    });
}

// 14. Removing a Contributor succeeds.
#[test]
fn test_remove_member_contributor_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let target_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, target_id, Role::Contributor)
            .await
            .expect("add failed");

        service
            .remove_member(owner_id, group_id, target_id)
            .await
            .expect("remove failed");

        let members = service
            .list_members(owner_id, group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 1);
    });
}

// 15. Removing the sole Group Admin is blocked.
#[test]
fn test_remove_member_sole_admin_conflict() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = service.remove_member(owner_id, group_id, owner_id).await;
        assert_conflict(result);
    });
}

// 16. A Contributor can leave a group.
#[test]
fn test_leave_group_contributor_succeeds() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let contributor_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add failed");

        service
            .leave_group(contributor_id, group_id)
            .await
            .expect("leave failed");

        let members = service
            .list_members(owner_id, group_id)
            .await
            .expect("list failed");
        assert_eq!(members.len(), 1);
    });
}

// 17. The sole Group Admin cannot leave.
#[test]
fn test_leave_group_sole_admin_conflict() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = service.leave_group(owner_id, group_id).await;
        assert_conflict(result);
    });
}

// 18. A non-member cannot list members.
#[test]
fn test_list_members_non_member_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = service.list_members(oid(), group_id).await;
        assert_forbidden(result);
    });
}

// 19. A Contributor cannot look up users for the group (checked before any
// user data is touched, so no seeded user is needed for this one).
#[test]
fn test_lookup_user_by_email_contributor_forbidden() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let contributor_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        service
            .add_member(owner_id, group_id, contributor_id, Role::Contributor)
            .await
            .expect("add failed");

        let result = service
            .lookup_user_by_email(contributor_id, group_id, "someone@example.com")
            .await;
        assert_forbidden(result);
    });
}

// 20. A Group Admin looking up an exact, existing email gets that user back.
#[test]
fn test_lookup_user_by_email_admin_finds_exact_match() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let email = unique_email("lookup-found");
        let target_id = seed_user(&email, "Found User").await;

        let found = service
            .lookup_user_by_email(owner_id, group_id, &email)
            .await
            .expect("lookup failed");
        assert_eq!(found.id, target_id.to_hex());
        assert_eq!(found.name, "Found User");
        assert_eq!(found.email, email);
    });
}

// 21. A Group Admin looking up an email with no match gets NotFound.
#[test]
fn test_lookup_user_by_email_no_match_not_found() {
    support::runtime().block_on(async {
        let service = setup().await;
        let owner_id = oid();
        let group = service
            .create_group(owner_id, "Team".to_string())
            .await
            .expect("create failed");
        let group_id = ObjectId::parse_str(&group.id).unwrap();

        let result = service
            .lookup_user_by_email(owner_id, group_id, &unique_email("lookup-missing"))
            .await;
        assert_not_found(result);
    });
}
