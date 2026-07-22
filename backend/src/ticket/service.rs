use chrono::DateTime;
use mongodb::{
    Database,
    bson::{self, DateTime as BsonDateTime, Document, oid::ObjectId},
};

use crate::errors::ApiError;
use crate::rbac::service::RbacService;
use crate::ticket::models::{
    CreateTicketInput, CreateTicketRequest, ListTicketsQuery, Ticket, TicketListResponse,
    TicketResponse, UpdateTicketRequest,
};
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;
use crate::utils::levenshtein_distance;

const DEFAULT_PER_PAGE: u64 = 20;
const MAX_PER_PAGE: u64 = 100;

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
        query: ListTicketsQuery,
    ) -> Result<TicketListResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;

        let creator = query
            .creator
            .as_deref()
            .map(ObjectId::parse_str)
            .transpose()
            .map_err(|_| ApiError::Validation("invalid creator id".to_string()))?;

        let mut tickets = self
            .repo
            .list_by_group(group_id, query.status, query.priority, creator)
            .await?;

        if let Some(term) = query.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            tickets = search_by_title(tickets, term);
        }

        let total = tickets.len() as u64;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE);
        let start = ((page - 1) * per_page) as usize;

        let mut items = Vec::new();
        for ticket in tickets.into_iter().skip(start).take(per_page as usize) {
            items.push(self.enrich_ticket(ticket).await?);
        }

        Ok(TicketListResponse {
            items,
            total,
            page,
            per_page,
        })
    }

    pub async fn update_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
        input: UpdateTicketRequest,
    ) -> Result<TicketResponse, ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;

        let mut changes = Document::new();
        if let Some(title) = input.title {
            changes.insert("title", title);
        }
        if let Some(description) = input.description {
            changes.insert("description", description);
        }
        if let Some(priority) = input.priority {
            changes.insert(
                "priority",
                bson::to_bson(&priority).expect("TicketPriority always serializes"),
            );
        }
        if let Some(status) = input.status {
            changes.insert(
                "status",
                bson::to_bson(&status).expect("TicketStatus always serializes"),
            );
        }
        changes.insert("updated_at", BsonDateTime::now());

        let ticket = self
            .repo
            .update_ticket(group_id, ticket_id, changes)
            .await?
            .ok_or(ApiError::NotFound)?;
        self.enrich_ticket(ticket).await
    }

    pub async fn delete_ticket(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        ticket_id: ObjectId,
    ) -> Result<(), ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;
        let deleted = self.repo.delete_ticket(group_id, ticket_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }
        Ok(())
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

// Case-insensitive substring match on title first; if that finds nothing,
// falls back to typo-tolerant matching (edit distance against each word in
// the title, so a typo in one word of a multi-word title still hits) —
// docs/api.md, "GET /groups/{id}/tickets" ("falls back to typo-tolerant
// similarity matching when the substring match returns nothing").
fn search_by_title(tickets: Vec<Ticket>, term: &str) -> Vec<Ticket> {
    let needle = term.to_lowercase();
    let substring_matches: Vec<Ticket> = tickets
        .iter()
        .filter(|t| t.title.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    if !substring_matches.is_empty() {
        return substring_matches;
    }

    // Allow roughly one edit per three characters of the query (min 1), so
    // short queries still require a near-exact match.
    let max_distance = (needle.chars().count() / 3).max(1);
    let mut scored: Vec<(usize, Ticket)> = tickets
        .into_iter()
        .filter_map(|t| {
            let best = t
                .title
                .to_lowercase()
                .split_whitespace()
                .map(|word| levenshtein_distance(word, &needle))
                .min()
                .unwrap_or(usize::MAX);
            (best <= max_distance).then_some((best, t))
        })
        .collect();
    scored.sort_by_key(|(distance, _)| *distance);
    scored.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use mongodb::bson::oid::ObjectId;

    use super::*;
    use crate::ticket::models::{TicketPriority, TicketStatus};

    fn ticket_with_title(title: &str) -> Ticket {
        let now = BsonDateTime::now();
        Ticket {
            id: Some(ObjectId::new()),
            group_id: ObjectId::new(),
            ticket_number: 1,
            title: title.to_string(),
            description: "description".to_string(),
            status: TicketStatus::Open,
            priority: TicketPriority::Low,
            created_by: ObjectId::new(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn search_by_title_prefers_substring_match() {
        let tickets = vec![ticket_with_title("Login bug"), ticket_with_title("API timeout")];
        let result = search_by_title(tickets, "login");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Login bug");
    }

    #[test]
    fn search_by_title_is_case_insensitive() {
        let tickets = vec![ticket_with_title("Login bug")];
        let result = search_by_title(tickets, "LOGIN");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn search_by_title_falls_back_to_typo_tolerant_match() {
        let tickets = vec![ticket_with_title("Login bug"), ticket_with_title("API timeout")];
        // "logn" has no substring match anywhere, but is one edit from "login".
        let result = search_by_title(tickets, "logn");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Login bug");
    }

    #[test]
    fn search_by_title_returns_nothing_when_too_dissimilar() {
        let tickets = vec![ticket_with_title("Login bug"), ticket_with_title("API timeout")];
        let result = search_by_title(tickets, "zzzzzzzz");
        assert!(result.is_empty());
    }
}
