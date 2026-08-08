use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

use crate::ticket::models::{TicketPriority, TicketStatus};

// The relation as stored — always from source_ticket_id's point of view.
// `RelatesTo` is symmetric: LinkService checks both directions before
// insert, so an A<->B relates_to pair is only ever one document, never two.
// `Blocks`/`Duplicates` are directional; the inverse reading is derived at
// read time (see LinkLabel) rather than stored as a second document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    Blocks,
    RelatesTo,
    Duplicates,
}

impl RelationType {
    // Resolves the label a viewer should see, given which side of the
    // stored relation their ticket is on.
    pub fn label_for(self, viewpoint_is_source: bool) -> LinkLabel {
        match (self, viewpoint_is_source) {
            (RelationType::Blocks, true) => LinkLabel::Blocks,
            (RelationType::Blocks, false) => LinkLabel::IsBlockedBy,
            (RelationType::RelatesTo, _) => LinkLabel::RelatesTo,
            (RelationType::Duplicates, true) => LinkLabel::Duplicates,
            (RelationType::Duplicates, false) => LinkLabel::IsDuplicatedBy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkLabel {
    Blocks,
    IsBlockedBy,
    RelatesTo,
    Duplicates,
    IsDuplicatedBy,
}

impl LinkLabel {
    // Short human-readable text for the activity log (see
    // LinkService::record_link_activity) — not used for the Links tab's own
    // rows, which render the label enum directly.
    pub fn describe(self) -> &'static str {
        match self {
            LinkLabel::Blocks => "blocks",
            LinkLabel::IsBlockedBy => "is blocked by",
            LinkLabel::RelatesTo => "relates to",
            LinkLabel::Duplicates => "duplicates",
            LinkLabel::IsDuplicatedBy => "is duplicated by",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketLink {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub source_ticket_id: ObjectId,
    pub target_ticket_id: ObjectId,
    pub relation_type: RelationType,
    pub created_by: ObjectId,
    pub created_at: BsonDateTime,
}

pub struct CreateLinkInput {
    pub group_id: ObjectId,
    pub source_ticket_id: ObjectId,
    pub target_ticket_id: ObjectId,
    pub relation_type: RelationType,
    pub created_by: ObjectId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLinkRequest {
    pub target_ticket_id: String,
    pub relation_type: RelationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketLinkResponse {
    pub id: String,
    pub group_id: String,
    // Already resolved for the ticket the request was scoped to — the
    // frontend never has to work out source-vs-target itself.
    pub label: LinkLabel,
    pub other_ticket_id: String,
    pub other_ticket_number: i64,
    pub other_ticket_title: String,
    pub other_ticket_status: TicketStatus,
    pub other_ticket_priority: TicketPriority,
    pub created_by: String,
    pub created_by_name: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RelationType::RelatesTo).unwrap(),
            "\"relates_to\""
        );
    }

    #[test]
    fn label_for_resolves_directional_relations() {
        assert_eq!(RelationType::Blocks.label_for(true), LinkLabel::Blocks);
        assert_eq!(RelationType::Blocks.label_for(false), LinkLabel::IsBlockedBy);
        assert_eq!(RelationType::Duplicates.label_for(true), LinkLabel::Duplicates);
        assert_eq!(
            RelationType::Duplicates.label_for(false),
            LinkLabel::IsDuplicatedBy
        );
    }

    #[test]
    fn label_for_relates_to_is_symmetric() {
        assert_eq!(RelationType::RelatesTo.label_for(true), LinkLabel::RelatesTo);
        assert_eq!(RelationType::RelatesTo.label_for(false), LinkLabel::RelatesTo);
    }
}
