use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::activity::models::{TicketActivity, TicketActivityResponse};
use crate::activity::repository::ActivityRepository;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

pub struct ActivityService {
    repo: ActivityRepository,
    ticket_repo: TicketRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl ActivityService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: ActivityRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Read-only for every group member (same visibility as the ticket itself
    // — this is history, not an admin action).
    pub async fn list_activity(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<TicketActivityResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        // Confirms ticket_id actually belongs to group_id before returning
        // anything — same cross-tenant guard as CommentService::
        // require_ticket_in_group (GroupScoped only proves membership in
        // group_id, not that ticket_id belongs to it).
        self.ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let entries = self.repo.list_by_ticket(group_id, ticket_id).await?;
        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            result.push(self.enrich(entry).await?);
        }
        Ok(result)
    }

    // TicketActivityResponse needs the actor's current display name, which
    // TicketActivity doesn't carry — mirrors CommentService::enrich_comment.
    // Resolved at read time (not snapshotted at write time) so a later name
    // change is reflected; falls back to empty string if the actor's account
    // was since deleted, same as enrich_comment/enrich_ticket.
    async fn enrich(&self, entry: TicketActivity) -> Result<TicketActivityResponse, ApiError> {
        let actor = self.user_service.find_by_id(entry.actor_id).await?;
        let actor_name = actor.map(|u| u.name).unwrap_or_default();
        Ok(TicketActivityResponse {
            id: entry.id.map(|id| id.to_hex()).unwrap_or_default(),
            group_id: entry.group_id.to_hex(),
            ticket_id: entry.ticket_id.to_hex(),
            actor_id: entry.actor_id.to_hex(),
            actor_name,
            event_type: entry.event_type,
            old_value: entry.old_value,
            new_value: entry.new_value,
            comment_id: entry.comment_id.map(|id| id.to_hex()),
            link_kind: entry.link_kind,
            occurred_at: DateTime::from_timestamp_millis(entry.occurred_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }
}
