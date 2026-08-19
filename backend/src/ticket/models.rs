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
    // Bumped only when title/description/priority change — NOT on a
    // status-only change (e.g. closing/reopening). This is what AiService
    // compares insights against (ai::models::AiTicketInsight), so closing a
    // ticket doesn't throw away a perfectly good cached summary/analysis:
    // the AI only ever reads title+description, so a status flip alone
    // doesn't make its output stale. Distinct from `updated_at`, which still
    // bumps on every edit including status.
    pub content_updated_at: BsonDateTime,
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

// All fields optional; the handler rejects a request where every field is
// absent (docs/api.md, "PATCH /groups/{id}/tickets/{ticket_id}").
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTicketRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<TicketPriority>,
    pub status: Option<TicketStatus>,
}

// Deserialized straight from the query string (web::Query), so every field is
// optional and `creator` stays a raw hex string (parsed to ObjectId in the
// service, same as other id path/body params).
#[derive(Debug, Deserialize)]
pub struct ListTicketsQuery {
    pub q: Option<String>,
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub creator: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TicketListResponse {
    pub items: Vec<TicketResponse>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
