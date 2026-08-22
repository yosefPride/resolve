use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::activity::models::{CreateActivityInput, TicketActivity};
use crate::utils::{RepoResult, insert_id};

// Shared infrastructure, not just ActivityService's backend: most writes come
// from *other* services (ticket/comment/link/reference record events;
// group/admin cascade deletes) — the service in this module only ever reads.
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
