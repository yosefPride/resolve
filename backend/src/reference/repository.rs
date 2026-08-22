use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::reference::models::{CreateReferenceInput, TicketReference};
use crate::utils::{RepoResult, insert_id};

pub struct ReferenceRepository {
    references: Collection<TicketReference>,
}

impl ReferenceRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            references: db.collection("ticket_references"),
        }
    }

    pub async fn insert(&self, input: CreateReferenceInput) -> RepoResult<TicketReference> {
        let reference = TicketReference {
            id: None,
            group_id: input.group_id,
            ticket_id: input.ticket_id,
            label: input.label,
            url: input.url,
            created_by: input.created_by,
            created_at: BsonDateTime::now(),
        };
        let id = insert_id(&self.references, &reference).await?;
        Ok(TicketReference {
            id: Some(id),
            ..reference
        })
    }

    pub async fn find_by_id(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        reference_id: ObjectId,
    ) -> RepoResult<Option<TicketReference>> {
        self.references
            .find_one(doc! { "_id": reference_id, "group_id": group_id, "ticket_id": ticket_id })
            .await
    }

    // Oldest first, same convention as CommentRepository::list_by_ticket.
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<Vec<TicketReference>> {
        self.references
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "created_at": 1 })
            .await?
            .try_collect()
            .await
    }

    pub async fn delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        reference_id: ObjectId,
    ) -> RepoResult<bool> {
        Ok(self
            .references
            .delete_one(doc! { "_id": reference_id, "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count
            > 0)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — same pattern as CommentRepository::delete_by_ticket.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<u64> {
        Ok(self
            .references
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data).
    pub async fn delete_by_group(&self, group_id: ObjectId) -> RepoResult<u64> {
        Ok(self
            .references
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count)
    }
}
