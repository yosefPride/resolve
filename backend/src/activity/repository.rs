use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::activity::models::{CreateActivityInput, TicketActivity};

#[derive(Debug)]
pub enum ActivityRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for ActivityRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for ActivityRepoError {}

impl From<mongodb::error::Error> for ActivityRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        ActivityRepoError::Database(err)
    }
}

pub struct ActivityRepository {
    activity: Collection<TicketActivity>,
}

impl ActivityRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            activity: db.collection("ticket_activity"),
        }
    }

    pub async fn insert(
        &self,
        input: CreateActivityInput,
    ) -> Result<TicketActivity, ActivityRepoError> {
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
        let result = self.activity.insert_one(&entry).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
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
    ) -> Result<Vec<TicketActivity>, ActivityRepoError> {
        let cursor = self
            .activity
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "occurred_at": -1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — same pattern as CommentRepository::delete_by_ticket.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, ActivityRepoError> {
        let result = self
            .activity
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data) — same
    // pattern as CommentRepository::delete_by_group.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, ActivityRepoError> {
        let result = self.activity.delete_many(doc! { "group_id": group_id }).await?;
        Ok(result.deleted_count)
    }
}
