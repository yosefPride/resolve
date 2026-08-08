use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{self, DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::link::models::{CreateLinkInput, RelationType, TicketLink};

#[derive(Debug)]
pub enum LinkRepoError {
    Duplicate,
    Database(mongodb::error::Error),
}

impl fmt::Display for LinkRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkRepoError::Duplicate => write!(f, "this link already exists"),
            LinkRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for LinkRepoError {}

impl From<mongodb::error::Error> for LinkRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        if is_duplicate_key(&err) {
            LinkRepoError::Duplicate
        } else {
            LinkRepoError::Database(err)
        }
    }
}

// Same pattern as GroupRepository's DuplicateMember mapping — the unique
// index on (group_id, source_ticket_id, target_ticket_id, relation_type)
// (db.rs) is the actual race guard; LinkService::create_link's pre-insert
// `exists` check just avoids hitting it in the common case.
fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    matches!(
        err.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000
    )
}

pub struct LinkRepository {
    links: Collection<TicketLink>,
}

impl LinkRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            links: db.collection("ticket_links"),
        }
    }

    pub async fn insert(&self, input: CreateLinkInput) -> Result<TicketLink, LinkRepoError> {
        let link = TicketLink {
            id: None,
            group_id: input.group_id,
            source_ticket_id: input.source_ticket_id,
            target_ticket_id: input.target_ticket_id,
            relation_type: input.relation_type,
            created_by: input.created_by,
            created_at: BsonDateTime::now(),
        };
        let result = self.links.insert_one(&link).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(TicketLink {
            id: Some(id),
            ..link
        })
    }

    pub async fn find_by_id(
        &self,
        group_id: ObjectId,
        link_id: ObjectId,
    ) -> Result<Option<TicketLink>, LinkRepoError> {
        Ok(self
            .links
            .find_one(doc! { "_id": link_id, "group_id": group_id })
            .await?)
    }

    // Every link touching this ticket, whichever side it's stored on,
    // oldest first (same convention as CommentRepository::list_by_ticket).
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<TicketLink>, LinkRepoError> {
        let cursor = self
            .links
            .find(doc! {
                "group_id": group_id,
                "$or": [
                    { "source_ticket_id": ticket_id },
                    { "target_ticket_id": ticket_id },
                ],
            })
            .sort(doc! { "created_at": 1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    // Pre-insert duplicate check for LinkService::create_link — checked in
    // both directions for RelatesTo (symmetric), only the exact direction
    // otherwise. The unique index in db.rs is the actual race guard; this
    // just turns the common case into a clean Conflict instead of a raw
    // duplicate-key error.
    pub async fn exists(
        &self,
        group_id: ObjectId,
        source_ticket_id: ObjectId,
        target_ticket_id: ObjectId,
        relation_type: RelationType,
    ) -> Result<bool, LinkRepoError> {
        let count = self
            .links
            .count_documents(doc! {
                "group_id": group_id,
                "source_ticket_id": source_ticket_id,
                "target_ticket_id": target_ticket_id,
                "relation_type": bson::to_bson(&relation_type).expect("RelationType always serializes"),
            })
            .await?;
        Ok(count > 0)
    }

    pub async fn delete(
        &self,
        group_id: ObjectId,
        link_id: ObjectId,
    ) -> Result<bool, LinkRepoError> {
        let result = self
            .links
            .delete_one(doc! { "_id": link_id, "group_id": group_id })
            .await?;
        Ok(result.deleted_count > 0)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — removes every link where the ticket appears on
    // either side.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, LinkRepoError> {
        let result = self
            .links
            .delete_many(doc! {
                "group_id": group_id,
                "$or": [
                    { "source_ticket_id": ticket_id },
                    { "target_ticket_id": ticket_id },
                ],
            })
            .await?;
        Ok(result.deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data).
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, LinkRepoError> {
        let result = self.links.delete_many(doc! { "group_id": group_id }).await?;
        Ok(result.deleted_count)
    }
}
