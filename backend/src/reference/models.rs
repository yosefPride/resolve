use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

// An external URL a user attaches to a ticket (a PR, a doc, a Slack thread)
// — distinct from link::models::TicketLink, which relates two tickets to
// each other. `label` is always populated: ReferenceService derives it from
// the URL's host when the caller leaves it blank (see
// ReferenceService::derive_label), so there's never an empty row to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketReference {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub label: String,
    pub url: String,
    pub created_by: ObjectId,
    pub created_at: BsonDateTime,
}

pub struct CreateReferenceInput {
    pub group_id: ObjectId,
    pub ticket_id: ObjectId,
    pub label: String,
    pub url: String,
    pub created_by: ObjectId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateReferenceRequest {
    // Blank/absent falls back to the URL's host (ReferenceService::
    // derive_label) rather than being rejected — a reference is still
    // useful without a hand-picked label.
    pub label: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketReferenceResponse {
    pub id: String,
    pub group_id: String,
    pub ticket_id: String,
    pub label: String,
    pub url: String,
    pub created_by: String,
    pub created_by_name: String,
    pub created_at: DateTime<Utc>,
}
