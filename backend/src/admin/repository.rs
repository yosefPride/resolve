use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{Document, doc, oid::ObjectId},
};

use crate::admin::models::AuditLogEntry;

#[derive(Debug)]
pub enum AdminRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for AdminRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdminRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for AdminRepoError {}

impl From<mongodb::error::Error> for AdminRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        AdminRepoError::Database(err)
    }
}

pub struct AdminRepository {
    audit_log: Collection<AuditLogEntry>,
}

impl AdminRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            audit_log: db.collection("admin_audit_log"),
        }
    }

    pub async fn insert_audit_entry(
        &self,
        entry: AuditLogEntry,
    ) -> Result<AuditLogEntry, AdminRepoError> {
        let result = self.audit_log.insert_one(&entry).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
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
    ) -> Result<Vec<AuditLogEntry>, AdminRepoError> {
        let mut filter = Document::new();
        if let Some(group_id) = group_id {
            filter.insert("group_id", group_id);
        }
        if let Some(deleted_user_id) = deleted_user_id {
            filter.insert("deleted_user_id", deleted_user_id);
        }
        let cursor = self
            .audit_log
            .find(filter)
            .sort(doc! { "created_at": -1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }
}
