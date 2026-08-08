use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::activity::models::{ActivityEventType, CreateActivityInput};
use crate::activity::repository::ActivityRepository;
use crate::comment::models::{Comment, CommentResponse, CreateCommentInput};
use crate::comment::repository::CommentRepository;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::models::{Ticket, TicketStatus};
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

pub struct CommentService {
    repo: CommentRepository,
    ticket_repo: TicketRepository,
    activity_repo: ActivityRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl CommentService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: CommentRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            activity_repo: ActivityRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Any group member may comment on a ticket (docs/specification/api.md,
    // "Comment Endpoints") — unlike tickets, there's no owner/admin split on
    // who may create one.
    pub async fn create_comment(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        content: String,
        parent_comment_id: Option<ObjectId>,
    ) -> Result<CommentResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        // Without this, a legitimate member of `group_id` could pass any
        // other group's ticket_id in the path and have a comment recorded
        // under their own group_id against a ticket that isn't actually in
        // it — a real cross-tenant leak found via live testing, not a
        // hypothetical. GroupScoped only proves membership in group_id; it
        // never checks that ticket_id belongs to that group.
        let ticket = self.require_ticket_in_group(group_id, ticket_id).await?;

        // A closed ticket is read-only for discussion: the thread stays fully
        // visible (list_comments is deliberately not gated on status) and
        // existing comments can still be deleted, but no new comment may be
        // added. Conflict rather than Forbidden — the caller has permission,
        // the ticket's state is what rejects this.
        if ticket.status == TicketStatus::Closed {
            return Err(ApiError::Conflict(
                "cannot comment on a closed ticket".to_string(),
            ));
        }

        if let Some(parent_id) = parent_comment_id {
            // The parent must actually be a comment on this same ticket —
            // otherwise a reply could dangle against an id from a different
            // ticket or group entirely.
            self.repo
                .find_by_id(group_id, ticket_id, parent_id)
                .await?
                .ok_or_else(|| {
                    ApiError::Validation("parent comment not found on this ticket".to_string())
                })?;
        }

        let comment = self
            .repo
            .insert_comment(CreateCommentInput {
                group_id,
                ticket_id,
                parent_comment_id,
                user_id,
                content,
            })
            .await?;
        self.activity_repo
            .insert(CreateActivityInput {
                group_id,
                ticket_id,
                actor_id: user_id,
                event_type: ActivityEventType::CommentAdded,
                old_value: None,
                new_value: None,
                comment_id: comment.id,
            })
            .await?;
        self.enrich_comment(comment).await
    }

    // Full thread in one response, oldest-first — no pagination (see
    // resolve-comments-feature-plan). The frontend builds the reply tree
    // client-side from each item's parent_comment_id.
    pub async fn list_comments(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<CommentResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.require_ticket_in_group(group_id, ticket_id).await?;
        let comments = self.repo.list_by_ticket(group_id, ticket_id).await?;
        let mut result = Vec::with_capacity(comments.len());
        for comment in comments {
            result.push(self.enrich_comment(comment).await?);
        }
        Ok(result)
    }

    // Owner or Group Admin only (RbacService::require_owner_or_group_admin).
    // A comment with existing replies is tombstoned (content replaced,
    // is_deleted set) rather than removed, so those replies keep a valid
    // parent to point at; a leaf comment (no replies) is hard-deleted.
    pub async fn delete_comment(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        comment_id: ObjectId,
    ) -> Result<(), ApiError> {
        let member = self.rbac.require_member(group_id, user_id).await?;
        let comment = self
            .repo
            .find_by_id(group_id, ticket_id, comment_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        RbacService::require_owner_or_group_admin(&member, comment.user_id)?;

        let has_replies = self.repo.has_replies(comment_id).await?;
        let changed = if has_replies {
            self.repo
                .soft_delete(group_id, ticket_id, comment_id)
                .await?
        } else {
            self.repo
                .hard_delete(group_id, ticket_id, comment_id)
                .await?
        };
        if !changed {
            return Err(ApiError::NotFound);
        }
        // Recorded for both the hard-delete and tombstone (soft-delete)
        // paths — either way the user's delete action succeeded, and the
        // activity log doesn't distinguish the two.
        self.activity_repo
            .insert(CreateActivityInput {
                group_id,
                ticket_id,
                actor_id: user_id,
                event_type: ActivityEventType::CommentDeleted,
                old_value: None,
                new_value: None,
                comment_id: Some(comment_id),
            })
            .await?;
        Ok(())
    }

    // Confirms ticket_id actually belongs to group_id before any comment read
    // or write against it — GroupScoped only proves the caller is a member of
    // group_id, it says nothing about which group ticket_id belongs to.
    // Returns the ticket so callers needing its status (create_comment) get it
    // from this same lookup rather than querying twice.
    async fn require_ticket_in_group(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Ticket, ApiError> {
        self.ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)
    }

    // CommentResponse needs the author's display name, which Comment doesn't
    // carry — mirrors TicketService::enrich_ticket. One find_by_id per
    // comment rather than a $lookup aggregation, same tradeoff made there.
    async fn enrich_comment(&self, comment: Comment) -> Result<CommentResponse, ApiError> {
        let author = self.user_service.find_by_id(comment.user_id).await?;
        let user_name = author.map(|u| u.name).unwrap_or_default();
        Ok(CommentResponse {
            id: comment.id.map(|id| id.to_hex()).unwrap_or_default(),
            group_id: comment.group_id.to_hex(),
            ticket_id: comment.ticket_id.to_hex(),
            parent_comment_id: comment.parent_comment_id.map(|id| id.to_hex()),
            user_id: comment.user_id.to_hex(),
            user_name,
            content: comment.content,
            is_deleted: comment.is_deleted,
            created_at: DateTime::from_timestamp_millis(comment.created_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }
}
