use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::activity::service::ActivityService;
use crate::errors::ApiError;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as comment_handlers::list_comments).
pub async fn list_activity(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = ActivityService::new(&state.db);
    let activity = service
        .list_activity(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(activity))
}
