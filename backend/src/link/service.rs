use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::activity::models::{ActivityEventType, CreateActivityInput, LinkKind};
use crate::activity::service::ActivityRepository;
use crate::errors::ApiError;
use crate::link::models::{
    CreateLinkInput, CreateLinkRequest, RelationType, TicketLink, TicketLinkResponse,
};
use crate::link::repository::LinkRepository;
use crate::rbac::service::RbacService;
use crate::ticket::models::Ticket;
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

pub struct LinkService {
    repo: LinkRepository,
    ticket_repo: TicketRepository,
    activity_repo: ActivityRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl LinkService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: LinkRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            activity_repo: ActivityRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Any group member may add a link — mirrors CommentService::create_comment,
    // no owner/admin split on who may create one.
    pub async fn create_link(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        input: CreateLinkRequest,
    ) -> Result<TicketLinkResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;

        let target_ticket_id = ObjectId::parse_str(&input.target_ticket_id)
            .map_err(|_| ApiError::Validation("invalid target_ticket_id".to_string()))?;
        if target_ticket_id == ticket_id {
            return Err(ApiError::Validation(
                "a ticket cannot be linked to itself".to_string(),
            ));
        }

        let source = self.require_ticket_in_group(group_id, ticket_id).await?;
        // target comes from the request body, not the path, so an
        // unknown/cross-group id is a 400 (bad input) rather than a 404 —
        // the ticket_id path segment is what owns the 404/NotFound meaning.
        let target = self
            .ticket_repo
            .find_by_id(group_id, target_ticket_id)
            .await?
            .ok_or_else(|| {
                ApiError::Validation("target ticket not found in this group".to_string())
            })?;

        if self
            .repo
            .exists(group_id, ticket_id, target_ticket_id, input.relation_type)
            .await?
        {
            return Err(ApiError::Conflict("this link already exists".to_string()));
        }
        // RelatesTo is symmetric — a single document represents the pair in
        // either direction, so the inverse must be checked too or the same
        // relation could be added twice, once from each ticket.
        if input.relation_type == RelationType::RelatesTo
            && self
                .repo
                .exists(group_id, target_ticket_id, ticket_id, RelationType::RelatesTo)
                .await?
        {
            return Err(ApiError::Conflict("this link already exists".to_string()));
        }

        let link = self
            .repo
            .insert(CreateLinkInput {
                group_id,
                source_ticket_id: ticket_id,
                target_ticket_id,
                relation_type: input.relation_type,
                created_by: user_id,
            })
            .await?;

        self.record_link_activity(
            group_id,
            &link,
            &source,
            &target,
            user_id,
            ActivityEventType::LinkAdded,
        )
        .await?;

        self.enrich(link, ticket_id, target).await
    }

    // Read-only for every group member.
    pub async fn list_links(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<Vec<TicketLinkResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        self.require_ticket_in_group(group_id, ticket_id).await?;

        let links = self.repo.list_by_ticket(group_id, ticket_id).await?;
        let mut result = Vec::with_capacity(links.len());
        for link in links {
            let other_ticket_id = if link.source_ticket_id == ticket_id {
                link.target_ticket_id
            } else {
                link.source_ticket_id
            };
            // The other ticket is guaranteed to exist and be in this group:
            // links are cascade-deleted the moment either side of them is
            // (LinkRepository::delete_by_ticket), so a link surviving here
            // always has both tickets still present.
            let other = self
                .ticket_repo
                .find_by_id(group_id, other_ticket_id)
                .await?
                .ok_or(ApiError::Internal)?;
            result.push(self.enrich(link, ticket_id, other).await?);
        }
        Ok(result)
    }

    // Owner or Group Admin only — mirrors CommentService::delete_comment.
    pub async fn delete_link(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        link_id: ObjectId,
    ) -> Result<(), ApiError> {
        let member = self.rbac.require_member(group_id, user_id).await?;
        let link = self
            .repo
            .find_by_id(group_id, link_id)
            .await?
            .filter(|l| l.source_ticket_id == ticket_id || l.target_ticket_id == ticket_id)
            .ok_or(ApiError::NotFound)?;
        RbacService::require_owner_or_group_admin(&member, link.created_by)?;

        let source = self
            .ticket_repo
            .find_by_id(group_id, link.source_ticket_id)
            .await?
            .ok_or(ApiError::Internal)?;
        let target = self
            .ticket_repo
            .find_by_id(group_id, link.target_ticket_id)
            .await?
            .ok_or(ApiError::Internal)?;

        let deleted = self.repo.delete(group_id, link_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }

        self.record_link_activity(
            group_id,
            &link,
            &source,
            &target,
            user_id,
            ActivityEventType::LinkRemoved,
        )
        .await?;
        Ok(())
    }

    // Logs one entry on each side of the link — being linked from another
    // ticket is a real change to *that* ticket's history too, not only the
    // one the request was made against.
    async fn record_link_activity(
        &self,
        group_id: ObjectId,
        link: &TicketLink,
        source: &Ticket,
        target: &Ticket,
        actor_id: ObjectId,
        event_type: ActivityEventType,
    ) -> Result<(), ApiError> {
        let source_text = format!(
            "{} #{}",
            link.relation_type.label_for(true).describe(),
            target.ticket_number
        );
        let target_text = format!(
            "{} #{}",
            link.relation_type.label_for(false).describe(),
            source.ticket_number
        );
        let (source_old, source_new, target_old, target_new) = match event_type {
            ActivityEventType::LinkAdded => (None, Some(source_text), None, Some(target_text)),
            _ => (Some(source_text), None, Some(target_text), None),
        };

        self.activity_repo
            .insert(CreateActivityInput {
                group_id,
                ticket_id: source.id.expect("ticket always has an id once loaded"),
                actor_id,
                event_type,
                old_value: source_old,
                new_value: source_new,
                comment_id: None,
                link_kind: Some(LinkKind::Relation),
            })
            .await?;
        self.activity_repo
            .insert(CreateActivityInput {
                group_id,
                ticket_id: target.id.expect("ticket always has an id once loaded"),
                actor_id,
                event_type,
                old_value: target_old,
                new_value: target_new,
                comment_id: None,
                link_kind: Some(LinkKind::Relation),
            })
            .await?;
        Ok(())
    }

    // TicketLinkResponse needs the other ticket's summary fields and the
    // creator's display name, neither of which TicketLink carries — mirrors
    // CommentService::enrich_comment.
    async fn enrich(
        &self,
        link: TicketLink,
        viewpoint_ticket_id: ObjectId,
        other: Ticket,
    ) -> Result<TicketLinkResponse, ApiError> {
        let is_source = link.source_ticket_id == viewpoint_ticket_id;
        let label = link.relation_type.label_for(is_source);
        let creator = self.user_service.find_by_id(link.created_by).await?;
        let created_by_name = creator.map(|u| u.name).unwrap_or_default();
        Ok(TicketLinkResponse {
            id: link.id.map(|id| id.to_hex()).unwrap_or_default(),
            group_id: link.group_id.to_hex(),
            label,
            other_ticket_id: other.id.map(|id| id.to_hex()).unwrap_or_default(),
            other_ticket_number: other.ticket_number,
            other_ticket_title: other.title,
            other_ticket_status: other.status,
            other_ticket_priority: other.priority,
            created_by: link.created_by.to_hex(),
            created_by_name,
            created_at: DateTime::from_timestamp_millis(link.created_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }

    // Confirms ticket_id actually belongs to group_id — same cross-tenant
    // guard as CommentService::require_ticket_in_group.
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
}
