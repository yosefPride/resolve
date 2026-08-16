use std::collections::BTreeMap;

use mongodb::{Database, bson::oid::ObjectId};

use crate::comment::repository::CommentRepository;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::reaction::models::{CommentReaction, ReactionSummary, SetReactionInput};
use crate::reaction::repository::ReactionRepository;

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

#[cfg(test)]
mod tests {
    use mongodb::bson::DateTime as BsonDateTime;

    use super::*;

    fn reaction(emoji: &str, user_id: ObjectId) -> CommentReaction {
        CommentReaction {
            id: None,
            group_id: ObjectId::new(),
            ticket_id: ObjectId::new(),
            comment_id: ObjectId::new(),
            user_id,
            emoji: emoji.to_string(),
            created_at: BsonDateTime::now(),
        }
    }

    #[test]
    fn summarize_reactions_groups_and_counts_by_emoji() {
        let viewer = ObjectId::new();
        let other = ObjectId::new();
        let rows = vec![
            reaction("\u{1F44D}", viewer),
            reaction("\u{1F44D}", other),
            reaction("\u{1F389}", other),
        ];
        let summary = summarize_reactions(&rows, viewer);

        let thumbs_up = summary.iter().find(|r| r.emoji == "\u{1F44D}").unwrap();
        assert_eq!(thumbs_up.count, 2);
        assert!(thumbs_up.reacted_by_me);

        let party = summary.iter().find(|r| r.emoji == "\u{1F389}").unwrap();
        assert_eq!(party.count, 1);
        assert!(!party.reacted_by_me);
    }

    #[test]
    fn summarize_reactions_empty_when_no_rows() {
        assert!(summarize_reactions(&[], ObjectId::new()).is_empty());
    }
}
