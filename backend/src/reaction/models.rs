use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

// One reaction per (comment_id, user_id) — enforced by the unique index in
// db.rs and by ReactionRepository::set_reaction's upsert-on-that-key, not by
// an app-level check-then-write. A user picking a new emoji replaces their
// existing reaction on this comment rather than adding a second one; that's
// the "one reaction per comment per user" rule (resolve-emoji-reactions-plan).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReaction {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub comment_id: ObjectId,
    pub user_id: ObjectId,
    pub emoji: String,
    pub created_at: BsonDateTime,
}

pub struct SetReactionInput {
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub comment_id: ObjectId,
    pub user_id: ObjectId,
    pub emoji: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetReactionRequest {
    pub emoji: String,
}

// Aggregated per-emoji view attached to CommentResponse.reactions — the
// client never sees individual reaction rows, only counts per emoji plus
// whether the requesting user is the one behind one of them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub reacted_by_me: bool,
}
