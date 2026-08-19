use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::reference::models::CreateReferenceRequest;
use crate::reference::service::ReferenceService;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

const MAX_URL_LEN: usize = 2000;
const MAX_LABEL_LEN: usize = 200;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

// No URL-parsing crate — same hand-rolled-over-dependency style as the rest
// of the codebase (e.g. utils::levenshtein_distance). Only a scheme check
// and a length cap: this is a link a human will click, not something parsed
// for routing, so "looks like an http(s) URL" is enough.
fn validate_create(input: &CreateReferenceRequest) -> Result<(), ApiError> {
    let url = input.url.trim();
    if url.is_empty() {
        return Err(ApiError::Validation("url is required".to_string()));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(ApiError::Validation(
            "url must start with http:// or https://".to_string(),
        ));
    }
    if url.chars().count() > MAX_URL_LEN {
        return Err(ApiError::Validation(format!(
            "url must be at most {MAX_URL_LEN} characters"
        )));
    }
    if let Some(label) = &input.label {
        if label.chars().count() > MAX_LABEL_LEN {
            return Err(ApiError::Validation(format!(
                "label must be at most {MAX_LABEL_LEN} characters"
            )));
        }
    }
    Ok(())
}

pub async fn create_reference(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<CreateReferenceRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let input = body.into_inner();
    validate_create(&input)?;

    let service = ReferenceService::new(&state.db);
    let reference = service
        .create_reference(scoped.user_id, scoped.group_id, ticket_id, input)
        .await?;
    Ok(HttpResponse::Created().json(reference))
}

pub async fn list_references(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = ReferenceService::new(&state.db);
    let references = service
        .list_references(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(references))
}

pub async fn delete_reference(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, reference_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let reference_id = parse_id(&reference_id)?;

    let service = ReferenceService::new(&state.db);
    service
        .delete_reference(scoped.user_id, scoped.group_id, ticket_id, reference_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
