use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::reference::models::{CreateReferenceInput, TicketReference};

#[derive(Debug)]
pub enum ReferenceRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for ReferenceRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for ReferenceRepoError {}

impl From<mongodb::error::Error> for ReferenceRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        ReferenceRepoError::Database(err)
    }
}

pub struct ReferenceRepository {
    references: Collection<TicketReference>,
}

impl ReferenceRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            references: db.collection("ticket_references"),
        }
    }

    pub async fn insert(
        &self,
        input: CreateReferenceInput,
    ) -> Result<TicketReference, ReferenceRepoError> {
        let reference = TicketReference {
            id: None,
            group_id: input.group_id,
            ticket_id: input.ticket_id,
            label: input.label,
            url: input.url,
            created_by: input.created_by,
            created_at: BsonDateTime::now(),
        };
        let result = self.references.insert_one(&reference).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
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
    ) -> Result<Option<TicketReference>, ReferenceRepoError> {
        Ok(self
            .references
            .find_one(doc! { "_id": reference_id, "group_id": group_id, "ticket_id": ticket_id })
            .await?)
    }

    // Oldest first, same convention as CommentRepository::list_by_ticket.
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<TicketReference>, ReferenceRepoError> {
        let cursor = self
            .references
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "created_at": 1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        reference_id: ObjectId,
    ) -> Result<bool, ReferenceRepoError> {
        let result = self
            .references
            .delete_one(
                doc! { "_id": reference_id, "group_id": group_id, "ticket_id": ticket_id },
            )
            .await?;
        Ok(result.deleted_count > 0)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — same pattern as CommentRepository::delete_by_ticket.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, ReferenceRepoError> {
        let result = self
            .references
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data).
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, ReferenceRepoError> {
        let result = self
            .references
            .delete_many(doc! { "group_id": group_id })
            .await?;
        Ok(result.deleted_count)
    }
}
