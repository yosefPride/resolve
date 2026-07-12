use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use resolve::admin::{
    models::{AuditAction, AuditLogEntry},
    repository::AdminRepository,
};

async fn setup() -> AdminRepository {
    dotenvy::dotenv().ok();
    let uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("failed to connect to MongoDB");
    let db = client.database("resolve_test");

    // Drop and recreate so each run starts from a known clean state.
    db.collection::<mongodb::bson::Document>("admin_audit_log")
        .drop()
        .await
        .expect("failed to drop admin_audit_log collection");

    AdminRepository::new(&db)
}

fn oid() -> ObjectId {
    ObjectId::new()
}

fn sample_entry(
    action: AuditAction,
    group_id: ObjectId,
    deleted_user_id: ObjectId,
    successor_user_id: Option<ObjectId>,
) -> AuditLogEntry {
    AuditLogEntry {
        id: None,
        action,
        group_id,
        deleted_user_id,
        successor_user_id,
        performed_by: oid(),
        created_at: BsonDateTime::now(),
    }
}

// 1. Insert an audit entry — id gets set, fields round-trip.
#[tokio::test]
async fn test_insert_audit_entry() {
    let repo = setup().await;
    let group_id = oid();
    let deleted_user_id = oid();
    let successor_id = oid();

    let inserted = repo
        .insert_audit_entry(sample_entry(
            AuditAction::Succession,
            group_id,
            deleted_user_id,
            Some(successor_id),
        ))
        .await
        .expect("insert failed");

    assert!(inserted.id.is_some());
    assert_eq!(inserted.action, AuditAction::Succession);
    assert_eq!(inserted.group_id, group_id);
    assert_eq!(inserted.deleted_user_id, deleted_user_id);
    assert_eq!(inserted.successor_user_id, Some(successor_id));

    // To see this for yourself: comment out the `.drop()` call in `setup()`
    // above, then run:
    //   cargo test test_insert_audit_entry -- --test-threads=1 --nocapture
    // and check the `admin_audit_log` collection in MongoDB Atlas.
}

// 2. A group_auto_deleted entry has no successor.
#[tokio::test]
async fn test_insert_audit_entry_group_auto_deleted() {
    let repo = setup().await;
    let inserted = repo
        .insert_audit_entry(sample_entry(AuditAction::GroupAutoDeleted, oid(), oid(), None))
        .await
        .expect("insert failed");

    assert_eq!(inserted.action, AuditAction::GroupAutoDeleted);
    assert!(inserted.successor_user_id.is_none());
}

// 3. list_audit_log_for_group returns only entries for that group.
#[tokio::test]
async fn test_list_audit_log_for_group() {
    let repo = setup().await;
    let group_id = oid();
    let other_group_id = oid();

    repo.insert_audit_entry(sample_entry(AuditAction::Succession, group_id, oid(), Some(oid())))
        .await
        .expect("insert failed");
    repo.insert_audit_entry(sample_entry(AuditAction::GroupAutoDeleted, group_id, oid(), None))
        .await
        .expect("insert failed");
    repo.insert_audit_entry(sample_entry(
        AuditAction::Succession,
        other_group_id,
        oid(),
        Some(oid()),
    ))
    .await
    .expect("insert failed");

    let entries = repo.list_audit_log_for_group(group_id).await.expect("list failed");
    assert_eq!(entries.len(), 2);
}

// 4. list_audit_log_for_user returns only entries for that deleted_user_id.
#[tokio::test]
async fn test_list_audit_log_for_user() {
    let repo = setup().await;
    let deleted_user_id = oid();

    repo.insert_audit_entry(sample_entry(
        AuditAction::Succession,
        oid(),
        deleted_user_id,
        Some(oid()),
    ))
    .await
    .expect("insert failed");
    repo.insert_audit_entry(sample_entry(
        AuditAction::GroupAutoDeleted,
        oid(),
        deleted_user_id,
        None,
    ))
    .await
    .expect("insert failed");
    repo.insert_audit_entry(sample_entry(AuditAction::Succession, oid(), oid(), Some(oid())))
        .await
        .expect("insert failed");

    let entries = repo
        .list_audit_log_for_user(deleted_user_id)
        .await
        .expect("list failed");
    assert_eq!(entries.len(), 2);
}
