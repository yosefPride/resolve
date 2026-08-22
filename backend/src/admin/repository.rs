use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{Document, doc, oid::ObjectId},
};

use crate::admin::models::AuditLogEntry;
use crate::utils::{RepoResult, insert_id};

pub struct AdminRepository {
    audit_log: Collection<AuditLogEntry>,
}

impl AdminRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            audit_log: db.collection("admin_audit_log"),
        }
    }

    pub async fn insert_audit_entry(&self, entry: AuditLogEntry) -> RepoResult<AuditLogEntry> {
        let id = insert_id(&self.audit_log, &entry).await?;
        Ok(AuditLogEntry {
            id: Some(id),
            ..entry
        })
    }

    // Returns entries newest-first. Each supplied filter narrows the result;
    // both absent returns the whole log. `group_id` and `deleted_user_id` each
    // have their own single-field index (see db.rs ensure_indexes).
    pub async fn list_audit_log(
        &self,
        group_id: Option<ObjectId>,
        deleted_user_id: Option<ObjectId>,
    ) -> RepoResult<Vec<AuditLogEntry>> {
        let mut filter = Document::new();
        if let Some(group_id) = group_id {
            filter.insert("group_id", group_id);
        }
        if let Some(deleted_user_id) = deleted_user_id {
            filter.insert("deleted_user_id", deleted_user_id);
        }
        self.audit_log
            .find(filter)
            .sort(doc! { "created_at": -1 })
            .await?
            .try_collect()
            .await
    }
}
