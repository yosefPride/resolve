use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{doc, oid::ObjectId},
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

    pub async fn list_audit_log_for_group(
        &self,
        group_id: ObjectId,
    ) -> Result<Vec<AuditLogEntry>, AdminRepoError> {
        let cursor = self.audit_log.find(doc! { "group_id": group_id }).await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn list_audit_log_for_user(
        &self,
        deleted_user_id: ObjectId,
    ) -> Result<Vec<AuditLogEntry>, AdminRepoError> {
        let cursor = self
            .audit_log
            .find(doc! { "deleted_user_id": deleted_user_id })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }
}
