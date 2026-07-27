use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::comment::models::CreateCommentRequest;
use crate::comment::service::CommentService;
use crate::errors::ApiError;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

const MAX_CONTENT_LEN: usize = 2000;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

// .chars().count(), not .len(): .len() counts UTF-8 bytes, which would wrongly
// reject valid Hebrew/non-Latin content under the nominal character limit —
// the same class of bug as the existing ticket-title length check.
fn validate_create(input: &CreateCommentRequest) -> Result<(), ApiError> {
    if input.content.trim().is_empty() {
        return Err(ApiError::Validation("content is required".to_string()));
    }
    if input.content.chars().count() > MAX_CONTENT_LEN {
        return Err(ApiError::Validation(format!(
            "content must be at most {MAX_CONTENT_LEN} characters"
        )));
    }
    Ok(())
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as ticket_handlers::get_ticket).
pub async fn create_comment(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<CreateCommentRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let input = body.into_inner();
    validate_create(&input)?;

    let parent_comment_id = input
        .parent_comment_id
        .as_deref()
        .map(parse_id)
        .transpose()?;

    let service = CommentService::new(&state.db);
    let comment = service
        .create_comment(
            scoped.user_id,
            scoped.group_id,
            ticket_id,
            input.content,
            parent_comment_id,
        )
        .await?;
    Ok(HttpResponse::Created().json(comment))
}

pub async fn list_comments(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = CommentService::new(&state.db);
    let comments = service
        .list_comments(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(comments))
}

pub async fn delete_comment(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, comment_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let comment_id = parse_id(&comment_id)?;

    let service = CommentService::new(&state.db);
    service
        .delete_comment(scoped.user_id, scoped.group_id, ticket_id, comment_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content: &str) -> CreateCommentRequest {
        CreateCommentRequest {
            content: content.to_string(),
            parent_comment_id: None,
        }
    }

    #[test]
    fn validate_create_rejects_empty_content() {
        let result = validate_create(&request("   "));
        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    #[test]
    fn validate_create_rejects_over_limit_by_char_count() {
        let too_long = "a".repeat(MAX_CONTENT_LEN + 1);
        let result = validate_create(&request(&too_long));
        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    // Regression guard for the .len() (byte-count) bug: each of these is a
    // 2-byte-in-UTF-8 Hebrew character, so a byte-based check would wrongly
    // reject this well under the nominal character limit.
    #[test]
    fn validate_create_accepts_hebrew_content_at_exact_char_limit() {
        let exactly_at_limit = "א".repeat(MAX_CONTENT_LEN);
        assert_eq!(exactly_at_limit.chars().count(), MAX_CONTENT_LEN);
        let result = validate_create(&request(&exactly_at_limit));
        assert!(result.is_ok());
    }
}
