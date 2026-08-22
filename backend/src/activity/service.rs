use chrono::DateTime;
use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::activity::models::{CreateActivityInput, TicketActivity, TicketActivityResponse};
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;
use crate::utils::{RepoResult, insert_id};

// Lives in this file rather than a separate repository.rs: the module is
// small, and the struct stays public because most writes come from *other*
// services (ticket/comment/link/reference record events; group/admin cascade
// deletes) — the service below only ever reads.
pub struct ActivityRepository {
    activity: Collection<TicketActivity>,
}

impl ActivityRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            activity: db.collection("ticket_activity"),
        }
    }

    pub async fn insert(&self, input: CreateActivityInput) -> RepoResult<TicketActivity> {
        let entry = TicketActivity {
            id: None,
            group_id: input.group_id,
            ticket_id: input.ticket_id,
            actor_id: input.actor_id,
            event_type: input.event_type,
            old_value: input.old_value,
            new_value: input.new_value,
            comment_id: input.comment_id,
            link_kind: input.link_kind,
            occurred_at: BsonDateTime::now(),
        };
        let id = insert_id(&self.activity, &entry).await?;
        Ok(TicketActivity {
            id: Some(id),
            ..entry
        })
    }

    // Newest-first, same convention as admin_audit_log — a timeline reads
    // most-recent-first.
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<Vec<TicketActivity>> {
        self.activity
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "occurred_at": -1 })
            .await?
            .try_collect()
            .await
    }

    // The single most recent entry across every ticket in the group — backs
    // the group list's "Last Activity" stat (GroupService::list_my_groups).
    // Needs its own (group_id, occurred_at) index (db.rs): the
    // (group_id, ticket_id, occurred_at) compound index used by
    // list_by_ticket can't serve a sort on occurred_at without also
    // constraining ticket_id, since ticket_id sits between the two fields.
    pub async fn find_latest_for_group(
        &self,
        group_id: ObjectId,
    ) -> RepoResult<Option<TicketActivity>> {
        self.activity
            .find_one(doc! { "group_id": group_id })
            .sort(doc! { "occurred_at": -1 })
            .await
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — same pattern as CommentRepository::delete_by_ticket.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<u64> {
        Ok(self
            .activity
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data) — same
    // pattern as CommentRepository::delete_by_group.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> RepoResult<u64> {
        Ok(self
            .activity
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count)
    }
}

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
