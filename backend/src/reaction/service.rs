use std::collections::BTreeMap;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::comment::repository::CommentRepository;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::reaction::models::{CommentReaction, ReactionSummary, SetReactionInput};
use crate::utils::RepoResult;

// Lives in this file rather than a separate repository.rs: the module is
// small, and the struct stays public because CommentService (enrich/cascade)
// and the ticket/group/admin cascade deletes call it directly.
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

pub struct ReactionService {
    repo: ReactionRepository,
    comment_repo: CommentRepository,
    rbac: RbacService,
}

impl ReactionService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: ReactionRepository::new(db),
            comment_repo: CommentRepository::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Any group member may react — same bar as CommentService::create_comment.
    // Deliberately no closed-ticket lock: unlike posting a new comment, a
    // reaction isn't new discussion, so a closed ticket's thread stays
    // reactable (resolve-emoji-reactions-plan).
    pub async fn set_reaction(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
        emoji: String,
    ) -> Result<Vec<ReactionSummary>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        // Filtered on all three of {_id, group_id, ticket_id}, so this alone
        // proves comment_id genuinely belongs to ticket_id within group_id —
        // the same cross-tenant guard CommentService::require_ticket_in_group
        // exists for, gotten here for free because a comment_id (unlike a
        // bare ticket_id) is enough to pin all three at once.
        self.comment_repo
            .find_by_id(group_id, ticket_id, comment_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        self.repo
            .set_reaction(SetReactionInput {
                group_id,
                ticket_id,
                comment_id,
                user_id,
                emoji,
            })
            .await?;
        self.summarize(comment_id, user_id).await
    }

    pub async fn remove_reaction(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> Result<Vec<ReactionSummary>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.comment_repo
            .find_by_id(group_id, ticket_id, comment_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        self.repo.remove_reaction(comment_id, user_id).await?;
        self.summarize(comment_id, user_id).await
    }

    async fn summarize(
        &self,
        comment_id: ObjectId,
        viewer_id: ObjectId,
    ) -> Result<Vec<ReactionSummary>, ApiError> {
        let rows = self.repo.list_by_comment(comment_id).await?;
        Ok(summarize_reactions(&rows, viewer_id))
    }
}

// Shared with CommentService::enrich_comment, which is why this is a free
// function rather than a private method here — a comment listing needs the
// same per-emoji aggregation as a single set/remove response, without
// depending on the whole ReactionService for it.
pub fn summarize_reactions(rows: &[CommentReaction], viewer_id: ObjectId) -> Vec<ReactionSummary> {
    // BTreeMap over HashMap: keeps emoji order stable across calls, so the
    // reaction bar doesn't visually reshuffle on every refetch.
    let mut counts: BTreeMap<&str, (i64, bool)> = BTreeMap::new();
    for row in rows {
        let entry = counts.entry(row.emoji.as_str()).or_insert((0, false));
        entry.0 += 1;
        if row.user_id == viewer_id {
            entry.1 = true;
        }
    }
    counts
        .into_iter()
        .map(|(emoji, (count, reacted_by_me))| ReactionSummary {
            emoji: emoji.to_string(),
            count,
            reacted_by_me,
        })
        .collect()
}
