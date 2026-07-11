use std::collections::HashMap;

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
    pub deleted_user_id: ObjectId,
    pub successor_user_id: Option<ObjectId>,
    pub performed_by: ObjectId,
    pub created_at: BsonDateTime,
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
