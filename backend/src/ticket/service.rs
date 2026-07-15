use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::models::{CreateTicketInput, CreateTicketRequest, Ticket, TicketResponse};
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

pub struct TicketService {
    repo: TicketRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl TicketService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: TicketRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    pub async fn create_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        input: CreateTicketRequest,
    ) -> Result<TicketResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;

        let ticket_number = self.repo.next_ticket_number(group_id).await?;
        let ticket = self
            .repo
            .insert_ticket(CreateTicketInput {
                group_id,
                ticket_number,
                title: input.title,
                description: input.description,
                priority: input.priority,
                created_by: user_id,
            })
            .await?;
        self.enrich_ticket(ticket).await
    }

    pub async fn get_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<TicketResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let ticket = self
            .repo
            .find_by_id(group_id, ticket_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        self.enrich_ticket(ticket).await
    }

    pub async fn list_tickets(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<Vec<TicketResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let tickets = self.repo.list_by_group(group_id).await?;
        let mut result = Vec::with_capacity(tickets.len());
        for ticket in tickets {
            result.push(self.enrich_ticket(ticket).await?);
        }
        Ok(result)
    }

    // TicketResponse needs the creator's display name, which Ticket doesn't
    // carry — mirrors GroupService::enrich_member. One find_by_id per ticket
    // rather than a $lookup aggregation, same tradeoff made there.
    async fn enrich_ticket(&self, ticket: Ticket) -> Result<TicketResponse, ApiError> {
        let creator = self.user_service.find_by_id(ticket.created_by).await?;
        let created_by_name = creator.map(|u| u.name).unwrap_or_default();
        Ok(TicketResponse {
            id: ticket.id.map(|id| id.to_hex()).unwrap_or_default(),
            group_id: ticket.group_id.to_hex(),
            ticket_number: ticket.ticket_number,
            title: ticket.title,
            description: ticket.description,
            status: ticket.status,
            priority: ticket.priority,
            created_by: ticket.created_by.to_hex(),
            created_by_name,
            created_at: DateTime::from_timestamp_millis(ticket.created_at.timestamp_millis())
                .unwrap_or_default(),
            updated_at: DateTime::from_timestamp_millis(ticket.updated_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }
}
