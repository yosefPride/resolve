use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::activity::models::{ActivityEventType, CreateActivityInput, LinkKind};
use crate::activity::service::ActivityRepository;
use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::reference::models::{
    CreateReferenceInput, CreateReferenceRequest, TicketReference, TicketReferenceResponse,
};
use crate::reference::repository::ReferenceRepository;
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

pub struct ReferenceService {
    repo: ReferenceRepository,
    ticket_repo: TicketRepository,
    activity_repo: ActivityRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl ReferenceService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: ReferenceRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            activity_repo: ActivityRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Any group member may attach a reference — mirrors
    // CommentService::create_comment, no owner/admin split on who may create
    // one. `url` is assumed already validated (handlers::validate_create).
    pub async fn create_reference(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        input: CreateReferenceRequest,
    ) -> Result<TicketReferenceResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.require_ticket_in_group(group_id, ticket_id).await?;

        let label = match input.label.as_deref().map(str::trim) {
            Some(label) if !label.is_empty() => label.to_string(),
            _ => derive_label(&input.url),
        };

        let reference = self
            .repo
            .insert(CreateReferenceInput {
                group_id,
                ticket_id,
                label,
                url: input.url,
                created_by: user_id,
            })
            .await?;

        self.record_reference_activity(
            group_id,
            ticket_id,
            user_id,
            ActivityEventType::LinkAdded,
            &reference.label,
        )
        .await?;

        self.enrich(reference).await
    }

    // Read-only for every group member.
    pub async fn list_references(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<TicketReferenceResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.require_ticket_in_group(group_id, ticket_id).await?;

        let references = self.repo.list_by_ticket(group_id, ticket_id).await?;
        let mut result = Vec::with_capacity(references.len());
        for reference in references {
            result.push(self.enrich(reference).await?);
        }
        Ok(result)
    }

    // Owner or Group Admin only — mirrors CommentService::delete_comment.
    pub async fn delete_reference(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        reference_id: ObjectId,
    ) -> Result<(), ApiError> {
        let member = self.rbac.require_member(group_id, user_id).await?;
        let reference = self
            .repo
            .find_by_id(group_id, ticket_id, reference_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        RbacService::require_owner_or_group_admin(&member, reference.created_by)?;

        let deleted = self.repo.delete(group_id, ticket_id, reference_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }

        self.record_reference_activity(
            group_id,
            ticket_id,
            user_id,
            ActivityEventType::LinkRemoved,
            &reference.label,
        )
        .await?;
        Ok(())
    }

    async fn record_reference_activity(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
        actor_id: ObjectId,
        event_type: ActivityEventType,
        label: &str,
    ) -> Result<(), ApiError> {
        let (old_value, new_value) = match event_type {
            ActivityEventType::LinkAdded => (None, Some(label.to_string())),
            _ => (Some(label.to_string()), None),
        };
        self.activity_repo
            .insert(CreateActivityInput {
                group_id,
                ticket_id,
                actor_id,
                event_type,
                old_value,
                new_value,
                comment_id: None,
                link_kind: Some(LinkKind::Reference),
            })
            .await?;
        Ok(())
    }

    // TicketReferenceResponse needs the creator's display name, which
    // TicketReference doesn't carry — mirrors CommentService::enrich_comment.
    async fn enrich(
        &self,
        reference: TicketReference,
    ) -> Result<TicketReferenceResponse, ApiError> {
        let creator = self.user_service.find_by_id(reference.created_by).await?;
        let created_by_name = creator.map(|u| u.name).unwrap_or_default();
        Ok(TicketReferenceResponse {
            id: reference.id.map(|id| id.to_hex()).unwrap_or_default(),
            group_id: reference.group_id.to_hex(),
            ticket_id: reference.ticket_id.to_hex(),
            label: reference.label,
            url: reference.url,
            created_by: reference.created_by.to_hex(),
            created_by_name,
            created_at: DateTime::from_timestamp_millis(reference.created_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }

    // Confirms ticket_id actually belongs to group_id — same cross-tenant
    // guard as CommentService::require_ticket_in_group.
    async fn require_ticket_in_group(
        &self,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<(), ApiError> {
        self.ticket_repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Ok(())
    }
}

// Fallback label when the caller leaves one blank — just the host, e.g.
// "github.com" from "https://github.com/org/repo/pull/12". Assumes `url`
// already passed handlers::validate_create's http(s):// check.
fn derive_label(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host_end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    without_scheme[..host_end].to_string()
}
