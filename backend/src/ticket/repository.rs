use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{self, DateTime as BsonDateTime, Document, doc, oid::ObjectId},
    options::ReturnDocument,
};

use crate::ticket::models::{CreateTicketInput, Ticket, TicketCounter, TicketPriority, TicketStatus};

#[derive(Debug)]
pub enum TicketRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for TicketRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TicketRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for TicketRepoError {}

impl From<mongodb::error::Error> for TicketRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        TicketRepoError::Database(err)
    }
}

pub struct TicketRepository {
    tickets: Collection<Ticket>,
    counters: Collection<TicketCounter>,
}

impl TicketRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            tickets: db.collection("tickets"),
            counters: db.collection("counters"),
        }
    }

    // Atomically allocates the next ticket_number for a group: upsert + $inc
    // means two tickets created in the same group at the same instant still
    // get distinct numbers, with no separate check-then-insert race
    // (docs/database.md, "counters").
    pub async fn next_ticket_number(&self, group_id: ObjectId) -> Result<i64, TicketRepoError> {
        let counter = self
            .counters
            .find_one_and_update(
                doc! { "_id": group_id },
                doc! { "$inc": { "ticket_seq": 1i64 } },
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await?
            .expect("upsert always returns a document");
        Ok(counter.ticket_seq)
    }

    pub async fn insert_ticket(&self, input: CreateTicketInput) -> Result<Ticket, TicketRepoError> {
        let now = BsonDateTime::now();
        let ticket = Ticket {
            id: None,
            group_id: input.group_id,
            ticket_number: input.ticket_number,
            title: input.title,
            description: input.description,
            status: TicketStatus::Open,
            priority: input.priority,
            created_by: input.created_by,
            created_at: now,
            updated_at: now,
        };
        let result = self.tickets.insert_one(&ticket).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(Ticket {
            id: Some(id),
            ..ticket
        })
    }

    // Filtered on group_id as well as _id, not just _id: this is what keeps a
    // ticket_id from one group unreadable through another group's id (a
    // mismatched pair simply finds nothing, per docs/database.md's
    // multi-tenancy rule).
    pub async fn find_by_id(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Option<Ticket>, TicketRepoError> {
        Ok(self
            .tickets
            .find_one(doc! { "_id": ticket_id, "group_id": group_id })
            .await?)
    }

    // status/priority/creator are exact-match, indexable fields, so they're
    // filtered here in Mongo; free-text title search (substring + typo-tolerant
    // fallback) has no Mongo-native equivalent and is done in-process by the
    // service over this method's result (see TicketService::list_tickets).
    pub async fn list_by_group(
        &self,
        group_id: ObjectId,
        status: Option<TicketStatus>,
        priority: Option<TicketPriority>,
        creator: Option<ObjectId>,
    ) -> Result<Vec<Ticket>, TicketRepoError> {
        let mut filter = doc! { "group_id": group_id };
        if let Some(status) = status {
            filter.insert(
                "status",
                bson::to_bson(&status).expect("TicketStatus always serializes"),
            );
        }
        if let Some(priority) = priority {
            filter.insert(
                "priority",
                bson::to_bson(&priority).expect("TicketPriority always serializes"),
            );
        }
        if let Some(creator) = creator {
            filter.insert("created_by", creator);
        }
        let cursor = self.tickets.find(filter).await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    // Filtered on group_id as well as _id, same isolation guarantee as
    // find_by_id: a ticket_id from another group simply matches nothing.
    pub async fn update_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        changes: Document,
    ) -> Result<Option<Ticket>, TicketRepoError> {
        Ok(self
            .tickets
            .find_one_and_update(
                doc! { "_id": ticket_id, "group_id": group_id },
                doc! { "$set": changes },
            )
            .return_document(ReturnDocument::After)
            .await?)
    }

    pub async fn delete_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<bool, TicketRepoError> {
        let result = self
            .tickets
            .delete_one(doc! { "_id": ticket_id, "group_id": group_id })
            .await?;
        Ok(result.deleted_count > 0)
    }

    // status is stored as its snake_case serialization ("open"/"closed"), so we
    // match the string directly rather than round-tripping through the enum.
    pub async fn count_open_by_group(&self, group_id: ObjectId) -> Result<u64, TicketRepoError> {
        Ok(self
            .tickets
            .count_documents(doc! { "group_id": group_id, "status": "open" })
            .await?)
    }
}
