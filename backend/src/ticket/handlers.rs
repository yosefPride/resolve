use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;
use crate::ticket::models::{CreateTicketRequest, ListTicketsQuery, UpdateTicketRequest};
use crate::ticket::service::TicketService;

const MAX_TITLE_LEN: usize = 200;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

fn validate_create(input: &CreateTicketRequest) -> Result<(), ApiError> {
    if input.title.trim().is_empty() {
        return Err(ApiError::Validation("title is required".to_string()));
    }
    if input.title.len() > MAX_TITLE_LEN {
        return Err(ApiError::Validation(format!(
            "title must be at most {MAX_TITLE_LEN} characters"
        )));
    }
    if input.description.trim().is_empty() {
        return Err(ApiError::Validation("description is required".to_string()));
    }
    Ok(())
}

fn validate_update(input: &UpdateTicketRequest) -> Result<(), ApiError> {
    if input.title.is_none()
        && input.description.is_none()
        && input.priority.is_none()
        && input.status.is_none()
    {
        return Err(ApiError::Validation(
            "at least one field is required".to_string(),
        ));
    }
    if let Some(title) = &input.title {
        if title.trim().is_empty() {
            return Err(ApiError::Validation("title must not be empty".to_string()));
        }
        if title.len() > MAX_TITLE_LEN {
            return Err(ApiError::Validation(format!(
                "title must be at most {MAX_TITLE_LEN} characters"
            )));
        }
    }
    if let Some(description) = &input.description {
        if description.trim().is_empty() {
            return Err(ApiError::Validation(
                "description must not be empty".to_string(),
            ));
        }
    }
    Ok(())
}

pub async fn create_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    body: web::Json<CreateTicketRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_create(&input)?;

    let service = TicketService::new(&state.db);
    let ticket = service
        .create_ticket(scoped.user_id, scoped.group_id, input)
        .await?;
    Ok(HttpResponse::Created().json(ticket))
}

pub async fn list_tickets(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    query: web::Query<ListTicketsQuery>,
) -> Result<HttpResponse, ApiError> {
    let service = TicketService::new(&state.db);
    let tickets = service
        .list_tickets(scoped.user_id, scoped.group_id, query.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(tickets))
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as group_handlers::update_member_role).
pub async fn get_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = TicketService::new(&state.db);
    let ticket = service
        .get_ticket(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(ticket))
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as get_ticket above).
pub async fn update_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateTicketRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let input = body.into_inner();
    validate_update(&input)?;

    let service = TicketService::new(&state.db);
    let ticket = service
        .update_ticket(scoped.user_id, scoped.group_id, ticket_id, input)
        .await?;
    Ok(HttpResponse::Ok().json(ticket))
}

pub async fn delete_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = TicketService::new(&state.db);
    service
        .delete_ticket(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
