use mongodb::{IndexModel, bson::doc, bson::oid::ObjectId, options::IndexOptions};
use resolve::group::{
    models::{CreateGroupInput, Role},
    repository::{GroupRepoError, GroupRepository},
};

async fn setup() -> GroupRepository {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("failed to connect to MongoDB");
    let db = client.database("resolve_test");

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

    GroupRepository::new(&db)
}

fn oid() -> ObjectId {
    ObjectId::new()
}

// 1. Create a group — id, name, owner_id, created_at are set.
#[tokio::test]
async fn test_create_group() {
    let repo = setup().await;
    let owner_id = oid();
    let group = repo
        .create_group(CreateGroupInput {
            name: "Acme".to_string(),
            owner_id,
        })
        .await
        .expect("create failed");

    assert!(group.id.is_some());
    assert_eq!(group.name, "Acme");
    assert_eq!(group.owner_id, owner_id);

    // To see this for yourself: comment out the next line and the two
    // `.drop()` calls in `setup()` above, then run:
    //   cargo test test_create_group -- --test-threads=1 --nocapture
    // and check the `groups` collection in MongoDB Atlas.
    repo.delete_group(group.id.unwrap()).await.ok();
}

// 2. Find group by id returns the correct group.
#[tokio::test]
async fn test_find_group_by_id_found() {
    let repo = setup().await;
    let group = repo
        .create_group(CreateGroupInput {
            name: "Findable".to_string(),
            owner_id: oid(),
        })
        .await
        .expect("create failed");

    let found = repo
        .find_group_by_id(group.id.unwrap())
        .await
        .expect("find failed")
        .expect("group not found");

    assert_eq!(found.name, "Findable");
    repo.delete_group(group.id.unwrap()).await.ok();
}

// 3. Find group by id returns None for an unknown id.
#[tokio::test]
async fn test_find_group_by_id_not_found() {
    let repo = setup().await;
    let result = repo.find_group_by_id(oid()).await.expect("find failed");
    assert!(result.is_none());
}

// 4. Insert member adds a member with the given role.
#[tokio::test]
async fn test_insert_member() {
    let repo = setup().await;
    let group_id = oid();
    let user_id = oid();

    let member = repo
        .insert_member(group_id, user_id, Role::GroupAdmin)
        .await
        .expect("insert failed");

    assert_eq!(member.group_id, group_id);
    assert_eq!(member.user_id, user_id);
    assert_eq!(member.role, Role::GroupAdmin);
}

// 5. Insert member twice for the same group+user is rejected (unique index).
#[tokio::test]
async fn test_insert_member_duplicate_rejected() {
    let repo = setup().await;
    let group_id = oid();
    let user_id = oid();

    repo.insert_member(group_id, user_id, Role::Contributor)
        .await
        .expect("first insert failed");

    let result = repo.insert_member(group_id, user_id, Role::Contributor).await;
    assert!(matches!(result, Err(GroupRepoError::DuplicateMember)));
}

// 6. Find member returns the correct membership.
#[tokio::test]
async fn test_find_member_found() {
    let repo = setup().await;
    let group_id = oid();
    let user_id = oid();
    repo.insert_member(group_id, user_id, Role::Contributor)
        .await
        .expect("insert failed");

    let found = repo
        .find_member(group_id, user_id)
        .await
        .expect("find failed")
        .expect("member not found");
    assert_eq!(found.role, Role::Contributor);
}

// 7. Find member returns None when not a member.
#[tokio::test]
async fn test_find_member_not_found() {
    let repo = setup().await;
    let result = repo.find_member(oid(), oid()).await.expect("find failed");
    assert!(result.is_none());
}

// 8. List members returns all members of a group, not other groups'.
#[tokio::test]
async fn test_list_members() {
    let repo = setup().await;
    let group_id = oid();
    let other_group_id = oid();

    repo.insert_member(group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");
    repo.insert_member(group_id, oid(), Role::Contributor)
        .await
        .expect("insert failed");
    repo.insert_member(other_group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");

    let members = repo.list_members(group_id).await.expect("list failed");
    assert_eq!(members.len(), 2);
}

// 9. List groups for user returns only the groups they belong to.
#[tokio::test]
async fn test_list_groups_for_user() {
    let repo = setup().await;
    let user_id = oid();

    let group_a = repo
        .create_group(CreateGroupInput {
            name: "A".to_string(),
            owner_id: user_id,
        })
        .await
        .expect("create failed");
    let group_b = repo
        .create_group(CreateGroupInput {
            name: "B".to_string(),
            owner_id: user_id,
        })
        .await
        .expect("create failed");
    let _group_c = repo
        .create_group(CreateGroupInput {
            name: "C".to_string(),
            owner_id: oid(),
        })
        .await
        .expect("create failed");

    repo.insert_member(group_a.id.unwrap(), user_id, Role::GroupAdmin)
        .await
        .expect("insert failed");
    repo.insert_member(group_b.id.unwrap(), user_id, Role::Contributor)
        .await
        .expect("insert failed");

    let groups = repo.list_groups_for_user(user_id).await.expect("list failed");
    assert_eq!(groups.len(), 2);
}

// 10. Count group admins reflects only GroupAdmin-role members.
#[tokio::test]
async fn test_count_group_admins() {
    let repo = setup().await;
    let group_id = oid();

    repo.insert_member(group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");
    repo.insert_member(group_id, oid(), Role::Contributor)
        .await
        .expect("insert failed");
    repo.insert_member(group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");

    let count = repo.count_group_admins(group_id).await.expect("count failed");
    assert_eq!(count, 2);
}

// 11. Update member role changes an existing member's role.
#[tokio::test]
async fn test_update_member_role() {
    let repo = setup().await;
    let group_id = oid();
    let user_id = oid();
    repo.insert_member(group_id, user_id, Role::Contributor)
        .await
        .expect("insert failed");

    let updated = repo
        .update_member_role(group_id, user_id, Role::GroupAdmin)
        .await
        .expect("update failed");
    assert!(updated);

    let member = repo
        .find_member(group_id, user_id)
        .await
        .expect("find failed")
        .expect("member not found");
    assert_eq!(member.role, Role::GroupAdmin);
}

// 12. Delete member removes the membership.
#[tokio::test]
async fn test_delete_member() {
    let repo = setup().await;
    let group_id = oid();
    let user_id = oid();
    repo.insert_member(group_id, user_id, Role::Contributor)
        .await
        .expect("insert failed");

    let deleted = repo.delete_member(group_id, user_id).await.expect("delete failed");
    assert!(deleted);

    let found = repo.find_member(group_id, user_id).await.expect("find failed");
    assert!(found.is_none());
}

// 13. Delete members by group removes all memberships for that group only.
#[tokio::test]
async fn test_delete_members_by_group() {
    let repo = setup().await;
    let group_id = oid();
    let other_group_id = oid();

    repo.insert_member(group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");
    repo.insert_member(group_id, oid(), Role::Contributor)
        .await
        .expect("insert failed");
    repo.insert_member(other_group_id, oid(), Role::GroupAdmin)
        .await
        .expect("insert failed");

    let deleted_count = repo
        .delete_members_by_group(group_id)
        .await
        .expect("delete failed");
    assert_eq!(deleted_count, 2);

    let remaining = repo.list_members(other_group_id).await.expect("list failed");
    assert_eq!(remaining.len(), 1);
}

// 14. Rename group updates the name.
#[tokio::test]
async fn test_rename_group() {
    let repo = setup().await;
    let group = repo
        .create_group(CreateGroupInput {
            name: "Old Name".to_string(),
            owner_id: oid(),
        })
        .await
        .expect("create failed");

    let renamed = repo
        .rename_group(group.id.unwrap(), "New Name".to_string())
        .await
        .expect("rename failed");
    assert!(renamed);

    let found = repo
        .find_group_by_id(group.id.unwrap())
        .await
        .expect("find failed")
        .expect("group not found");
    assert_eq!(found.name, "New Name");
}

// 15. Delete group removes the group document.
#[tokio::test]
async fn test_delete_group() {
    let repo = setup().await;
    let group = repo
        .create_group(CreateGroupInput {
            name: "ToDelete".to_string(),
            owner_id: oid(),
        })
        .await
        .expect("create failed");

    let deleted = repo.delete_group(group.id.unwrap()).await.expect("delete failed");
    assert!(deleted);

    let found = repo.find_group_by_id(group.id.unwrap()).await.expect("find failed");
    assert!(found.is_none());
}

// 16. List all groups returns every group, regardless of owner.
#[tokio::test]
async fn test_list_all_groups() {
    let repo = setup().await;
    repo.create_group(CreateGroupInput {
        name: "A".to_string(),
        owner_id: oid(),
    })
    .await
    .expect("create failed");
    repo.create_group(CreateGroupInput {
        name: "B".to_string(),
        owner_id: oid(),
    })
    .await
    .expect("create failed");
    repo.create_group(CreateGroupInput {
        name: "C".to_string(),
        owner_id: oid(),
    })
    .await
    .expect("create failed");

    let groups = repo.list_all_groups().await.expect("list failed");
    assert_eq!(groups.len(), 3);
}
