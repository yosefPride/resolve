use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

// One entry per discrete change — an update touching status, priority, and
// title in a single PATCH writes three of these, not one blob, so the
// Activity tab reads as granular events ("priority changed low -> high")
// instead of an opaque "ticket updated". Read-only from the API's point of
// view: entries are written exclusively by TicketService/CommentService as a
// side effect of their own mutations, never accepted from a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventType {
    TicketCreated,
    StatusChanged,
    PriorityChanged,
    TitleChanged,
    DescriptionChanged,
    CommentAdded,
    CommentDeleted,
    LinkAdded,
    LinkRemoved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketActivity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub actor_id: ObjectId,
    pub event_type: ActivityEventType,
    // Only populated for status_changed/priority_changed/title_changed.
    // Deliberately absent for description_changed: the text can be long and a
    // raw before/after dump isn't useful in a timeline, so only the fact that
    // it changed is recorded.
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    // Only populated for comment_added/comment_deleted. No comment content is
    // ever copied here — that stays exclusively in the Comments tab.
    pub comment_id: Option<ObjectId>,
    pub occurred_at: BsonDateTime,
}

pub struct CreateActivityInput {
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub actor_id: ObjectId,
    pub event_type: ActivityEventType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub comment_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketActivityResponse {
    pub id: String,
    pub group_id: String,
    pub ticket_id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub event_type: ActivityEventType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub comment_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_event_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ActivityEventType::TicketCreated).unwrap(),
            "\"ticket_created\""
        );
        assert_eq!(
            serde_json::to_string(&ActivityEventType::StatusChanged).unwrap(),
            "\"status_changed\""
        );
        assert_eq!(
            serde_json::to_string(&ActivityEventType::CommentDeleted).unwrap(),
            "\"comment_deleted\""
        );
        assert_eq!(
            serde_json::to_string(&ActivityEventType::LinkAdded).unwrap(),
            "\"link_added\""
        );
    }
}
