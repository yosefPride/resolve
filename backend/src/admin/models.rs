use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

use crate::group::models::MemberResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Succession,
    GroupAutoDeleted,
}

// One row per succession or auto-deletion performed during an admin-triggered
// user deletion — see docs/database.md ("admin_audit_log") and docs/rbac.md
// ("Group Admin Succession"). Not group-scoped tenant data: written by System
// Admin, not read by group-scoped business logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub action: AuditAction,
    pub group_id: ObjectId,
    // Names are snapshotted at write time: by the time the log is read, the
    // deleted user (always) and an auto-deleted group (when action =
    // group_auto_deleted) no longer exist, so their ids can't be resolved later.
    // `default` so entries written before this field existed still deserialize
    // (they read back with an empty name rather than failing the whole query).
    #[serde(default)]
    pub group_name: String,
    pub deleted_user_id: ObjectId,
    #[serde(default)]
    pub deleted_user_name: String,
    pub successor_user_id: Option<ObjectId>,
    #[serde(default)]
    pub successor_user_name: Option<String>,
    pub performed_by: ObjectId,
    #[serde(default)]
    pub performed_by_name: String,
    pub created_at: BsonDateTime,
}

// Client-facing shape of an AuditLogEntry: ObjectIds rendered as hex strings
// and the timestamp as an RFC3339 DateTime, matching GroupResponse/MemberResponse.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntryResponse {
    pub id: String,
    pub action: AuditAction,
    pub group_id: String,
    pub group_name: String,
    pub deleted_user_id: String,
    pub deleted_user_name: String,
    pub successor_user_id: Option<String>,
    pub successor_user_name: Option<String>,
    pub performed_by: String,
    pub performed_by_name: String,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogEntry> for AuditLogEntryResponse {
    fn from(entry: AuditLogEntry) -> Self {
        AuditLogEntryResponse {
            id: entry.id.map(|id| id.to_hex()).unwrap_or_default(),
            action: entry.action,
            group_id: entry.group_id.to_hex(),
            group_name: entry.group_name,
            deleted_user_id: entry.deleted_user_id.to_hex(),
            deleted_user_name: entry.deleted_user_name,
            successor_user_id: entry.successor_user_id.map(|id| id.to_hex()),
            successor_user_name: entry.successor_user_name,
            performed_by: entry.performed_by.to_hex(),
            performed_by_name: entry.performed_by_name,
            created_at: DateTime::from_timestamp_millis(entry.created_at.timestamp_millis())
                .unwrap_or_default(),
        }
    }
}

// Optional filters for GET /admin/audit-log. Both absent = the full log,
// newest-first. `user_id` filters on the *deleted* user.
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub group_id: Option<String>,
    pub user_id: Option<String>,
}

// Query params shared by the admin list endpoints (GET /admin/users and
// GET /admin/groups). `search` is a case-insensitive substring match; absent or
// blank returns the full list. Grows a page/per_page pair when pagination lands.
#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockedGroupInfo {
    pub group_id: String,
    pub group_name: String,
    pub eligible_successors: Vec<MemberResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoDeleteGroupInfo {
    pub group_id: String,
    pub group_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletionCheckResponse {
    pub blocked_groups: Vec<BlockedGroupInfo>,
    pub auto_delete_groups: Vec<AutoDeleteGroupInfo>,
}

// group_id (hex string) -> successor user_id (hex string). Only groups
// appearing in DeletionCheckResponse::blocked_groups need an entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteUserRequest {
    pub successors: HashMap<String, String>,
}
