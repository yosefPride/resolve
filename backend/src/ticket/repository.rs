use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
    options::ReturnDocument,
};

use crate::ticket::models::{CreateTicketInput, Ticket, TicketCounter, TicketStatus};

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

    pub async fn list_by_group(&self, group_id: ObjectId) -> Result<Vec<Ticket>, TicketRepoError> {
        let cursor = self.tickets.find(doc! { "group_id": group_id }).await?;
        cursor.try_collect().await.map_err(Into::into)
    }
}
