use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::comment::models::{Comment, CreateCommentInput, DELETED_CONTENT_PLACEHOLDER};

#[derive(Debug)]
pub enum CommentRepoError {
    Database(mongodb::error::Error),
}

impl fmt::Display for CommentRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommentRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for CommentRepoError {}

impl From<mongodb::error::Error> for CommentRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        CommentRepoError::Database(err)
    }
}

pub struct CommentRepository {
    comments: Collection<Comment>,
}

impl CommentRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            comments: db.collection("comments"),
        }
    }

    pub async fn insert_comment(
        &self,
        input: CreateCommentInput,
    ) -> Result<Comment, CommentRepoError> {
        let comment = Comment {
            id: None,
            group_id: input.group_id,
            ticket_id: input.ticket_id,
            parent_comment_id: input.parent_comment_id,
            user_id: input.user_id,
            content: input.content,
            is_deleted: false,
            created_at: BsonDateTime::now(),
        };
        let result = self.comments.insert_one(&comment).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(Comment {
            id: Some(id),
            ..comment
        })
    }

    // Filtered on group_id and ticket_id as well as _id, same multi-tenancy
    // pattern as TicketRepository::find_by_id: a comment_id from another
    // group/ticket simply matches nothing.
    pub async fn find_by_id(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> Result<Option<Comment>, CommentRepoError> {
        Ok(self
            .comments
            .find_one(doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id })
            .await?)
    }

    // Every comment for a ticket, oldest first — no pagination, a discussion
    // thread is read in full (resolve-comments-feature-plan). Includes
    // soft-deleted (tombstoned) comments so their surviving replies still
    // have a parent to render against.
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<Comment>, CommentRepoError> {
        let cursor = self
            .comments
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "created_at": 1 })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    // Whether any other comment replies to this one — decides hard vs. soft
    // delete in CommentService::delete_comment.
    pub async fn has_replies(&self, comment_id: ObjectId) -> Result<bool, CommentRepoError> {
        let count = self
            .comments
            .count_documents(doc! { "parent_comment_id": comment_id })
            .await?;
        Ok(count > 0)
    }

    pub async fn hard_delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> Result<bool, CommentRepoError> {
        let result = self
            .comments
            .delete_one(doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count > 0)
    }

    // Tombstone: keeps the document (so replies' parent_comment_id stays
    // valid) but clears the visible content and flips is_deleted.
    pub async fn soft_delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> Result<bool, CommentRepoError> {
        let result = self
            .comments
            .update_one(
                doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id },
                doc! { "$set": { "is_deleted": true, "content": DELETED_CONTENT_PLACEHOLDER } },
            )
            .await?;
        Ok(result.modified_count > 0)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket). Unconditional hard delete of every comment on the
    // ticket, tombstones included — the ticket itself is gone, so there is
    // nothing left for a reply to stay valid against. Filtered on group_id as
    // well as ticket_id so the query uses the (group_id, ticket_id) index
    // (ticket_id alone isn't a prefix of it).
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<u64, CommentRepoError> {
        let result = self
            .comments
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?;
        Ok(result.deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data). Same
    // unconditional hard delete as delete_by_ticket, scoped to group_id
    // instead — comments carry their own group_id, so no per-ticket fan-out
    // is needed.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> Result<u64, CommentRepoError> {
        let result = self
            .comments
            .delete_many(doc! { "group_id": group_id })
            .await?;
        Ok(result.deleted_count)
    }
}
