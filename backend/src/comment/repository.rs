use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::comment::models::{Comment, CreateCommentInput, DELETED_CONTENT_PLACEHOLDER};
use crate::utils::{RepoResult, insert_id};

pub struct CommentRepository {
    comments: Collection<Comment>,
}

impl CommentRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            comments: db.collection("comments"),
        }
    }

    pub async fn insert_comment(&self, input: CreateCommentInput) -> RepoResult<Comment> {
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
        let id = insert_id(&self.comments, &comment).await?;
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
    ) -> RepoResult<Option<Comment>> {
        self.comments
            .find_one(doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id })
            .await
    }

    // Every comment for a ticket, oldest first — no pagination, a discussion
    // thread is read in full (resolve-comments-feature-plan). Includes
    // soft-deleted (tombstoned) comments so their surviving replies still
    // have a parent to render against.
    pub async fn list_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<Vec<Comment>> {
        self.comments
            .find(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .sort(doc! { "created_at": 1 })
            .await?
            .try_collect()
            .await
    }

    // Whether any other comment replies to this one — decides hard vs. soft
    // delete in CommentService::delete_comment.
    pub async fn has_replies(&self, comment_id: ObjectId) -> RepoResult<bool> {
        Ok(self
            .comments
            .count_documents(doc! { "parent_comment_id": comment_id })
            .await?
            > 0)
    }

    pub async fn hard_delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> RepoResult<bool> {
        Ok(self
            .comments
            .delete_one(doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count
            > 0)
    }

    // Tombstone: keeps the document (so replies' parent_comment_id stays
    // valid) but clears the visible content and flips is_deleted.
    pub async fn soft_delete(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> RepoResult<bool> {
        Ok(self
            .comments
            .update_one(
                doc! { "_id": comment_id, "group_id": group_id, "ticket_id": ticket_id },
                doc! { "$set": { "is_deleted": true, "content": DELETED_CONTENT_PLACEHOLDER } },
            )
            .await?
            .modified_count
            > 0)
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
    ) -> RepoResult<u64> {
        Ok(self
            .comments
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data). Same
    // unconditional hard delete as delete_by_ticket, scoped to group_id
    // instead — comments carry their own group_id, so no per-ticket fan-out
    // is needed.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> RepoResult<u64> {
        Ok(self
            .comments
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count)
    }
}
