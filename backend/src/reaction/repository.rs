use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::reaction::models::{CommentReaction, SetReactionInput};
use crate::utils::RepoResult;

pub struct ReactionRepository {
    reactions: Collection<CommentReaction>,
}

impl ReactionRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            reactions: db.collection("comment_reactions"),
        }
    }

    // Upsert on the unique (comment_id, user_id) key: a user picking a new
    // emoji replaces whatever reaction they already had on this comment
    // rather than adding a second row. The unique index in db.rs is the
    // actual one-reaction-per-user guard; this filter is what makes the
    // write an update-in-place instead of a duplicate-key error.
    pub async fn set_reaction(&self, input: SetReactionInput) -> RepoResult<()> {
        let reaction = CommentReaction {
            id: None,
            group_id: input.group_id,
            ticket_id: input.ticket_id,
            comment_id: input.comment_id,
            user_id: input.user_id,
            emoji: input.emoji,
            created_at: BsonDateTime::now(),
        };
        self.reactions
            .replace_one(
                doc! { "comment_id": input.comment_id, "user_id": input.user_id },
                &reaction,
            )
            .upsert(true)
            .await?;
        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        comment_id: ObjectId,
        user_id: ObjectId,
    ) -> RepoResult<bool> {
        Ok(self
            .reactions
            .delete_one(doc! { "comment_id": comment_id, "user_id": user_id })
            .await?
            .deleted_count
            > 0)
    }

    // Every reaction on one comment, for building its ReactionSummary list.
    // Called once per comment from CommentService::enrich_comment — the same
    // one-query-per-row tradeoff that module already accepts for user_name.
    pub async fn list_by_comment(&self, comment_id: ObjectId) -> RepoResult<Vec<CommentReaction>> {
        self.reactions
            .find(doc! { "comment_id": comment_id })
            .await?
            .try_collect()
            .await
    }

    // Cascade target for a single comment's deletion (CommentService::
    // delete_comment), on both the hard-delete and tombstone branches — a
    // tombstoned comment has nothing left worth reacting to, so its
    // reactions are cleared right along with a hard-deleted one's. Not
    // group/ticket-filtered: only ever called on a comment_id already
    // resolved through CommentRepository::find_by_id, same as
    // CommentRepository::has_replies.
    pub async fn delete_by_comment(&self, comment_id: ObjectId) -> RepoResult<u64> {
        Ok(self
            .reactions
            .delete_many(doc! { "comment_id": comment_id })
            .await?
            .deleted_count)
    }

    // Cascade target for a single ticket's deletion (TicketService::
    // delete_ticket) — mirrors CommentRepository::delete_by_ticket, filtered
    // the same way so it can use the (group_id, ticket_id) prefix.
    pub async fn delete_by_ticket(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> RepoResult<u64> {
        Ok(self
            .reactions
            .delete_many(doc! { "group_id": group_id, "ticket_id": ticket_id })
            .await?
            .deleted_count)
    }

    // Cascade target for a whole group's deletion (purge_group_data) —
    // mirrors CommentRepository::delete_by_group.
    pub async fn delete_by_group(&self, group_id: ObjectId) -> RepoResult<u64> {
        Ok(self
            .reactions
            .delete_many(doc! { "group_id": group_id })
            .await?
            .deleted_count)
    }
}
