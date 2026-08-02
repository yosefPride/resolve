use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

// Content is replaced with DELETED_CONTENT_PLACEHOLDER and is_deleted flips to
// true when a comment with existing replies is deleted (CommentService::
// delete_comment) — the document stays so those replies keep a valid
// parent_comment_id to point at, instead of referencing a vanished id. A
// leaf comment (no replies) is hard-deleted instead of tombstoned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub parent_comment_id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub content: String,
    pub is_deleted: bool,
    pub created_at: BsonDateTime,
}

pub struct CreateCommentInput {
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub parent_comment_id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub content: String,
}

pub const DELETED_CONTENT_PLACEHOLDER: &str = "[comment deleted]";

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_comment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: String,
    pub group_id: String,
    pub ticket_id: String,
    pub parent_comment_id: Option<String>,
    pub user_id: String,
    pub user_name: String,
    pub content: String,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}
