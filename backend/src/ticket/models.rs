use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriority {
    Low,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    // Running number scoped to group_id (1, 2, 3, ... independent per group),
    // allocated atomically via TicketRepository::next_ticket_number. Distinct
    // from `_id`: this is the human-facing number shown in the UI.
    pub ticket_number: i64,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub created_by: ObjectId,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

pub struct CreateTicketInput {
    pub group_id: ObjectId,
    pub ticket_number: i64,
    pub title: String,
    pub description: String,
    pub priority: TicketPriority,
    pub created_by: ObjectId,
}

// Backs the per-group ticket_number sequence (docs/database.md, "counters").
// One document per group, keyed by group_id as _id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketCounter {
    #[serde(rename = "_id")]
    pub group_id: ObjectId,
    pub ticket_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketResponse {
    pub id: String,
    pub group_id: String,
    pub ticket_number: i64,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub created_by: String,
    pub created_by_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTicketRequest {
    pub title: String,
    pub description: String,
    pub priority: TicketPriority,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TicketStatus::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&TicketStatus::Closed).unwrap(),
            "\"closed\""
        );
    }

    #[test]
    fn ticket_status_rejects_unknown_value() {
        let result: Result<TicketStatus, _> = serde_json::from_str("\"in_progress\"");
        assert!(result.is_err());
    }

    #[test]
    fn ticket_priority_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TicketPriority::Low).unwrap(),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&TicketPriority::High).unwrap(),
            "\"high\""
        );
        assert_eq!(
            serde_json::to_string(&TicketPriority::Critical).unwrap(),
            "\"critical\""
        );
    }

    #[test]
    fn ticket_priority_rejects_unknown_value() {
        let result: Result<TicketPriority, _> = serde_json::from_str("\"medium\"");
        assert!(result.is_err());
    }
}
