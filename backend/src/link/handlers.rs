use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::link::models::CreateLinkRequest;
use crate::link::service::LinkService;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as comment_handlers::create_comment).
pub async fn create_link(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<CreateLinkRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = LinkService::new(&state.db);
    let link = service
        .create_link(scoped.user_id, scoped.group_id, ticket_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(link))
}

pub async fn list_links(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = LinkService::new(&state.db);
    let links = service
        .list_links(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(links))
}

pub async fn delete_link(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, link_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let link_id = parse_id(&link_id)?;

    let service = LinkService::new(&state.db);
    service
        .delete_link(scoped.user_id, scoped.group_id, ticket_id, link_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
