use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::reaction::models::SetReactionRequest;
use crate::reaction::service::ReactionService;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

// Generous relative to a real emoji (almost always 1-4 UTF-16 code units,
// 1-2 chars): covers ZWJ sequences and skin-tone/flag modifiers, which can
// run to several chars, without accepting an arbitrary sentence. Format-only
// — same division of labor as comment::handlers::validate_create, where the
// handler checks shape and the service checks existence/permission.
const MAX_EMOJI_CHARS: usize = 8;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

fn validate_emoji(input: &SetReactionRequest) -> Result<String, ApiError> {
    let trimmed = input.emoji.trim();
    if trimmed.is_empty() {
        return Err(ApiError::Validation("emoji is required".to_string()));
    }
    if trimmed.chars().count() > MAX_EMOJI_CHARS {
        return Err(ApiError::Validation("emoji is too long".to_string()));
    }
    Ok(trimmed.to_string())
}

// GroupScoped consumes {id}; web::Path still extracts all four segments, so
// the first is dropped in favor of scoped.group_id — same pattern as
// comment_handlers::create_comment.
pub async fn set_reaction(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
    body: web::Json<SetReactionRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, comment_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let comment_id = parse_id(&comment_id)?;
    let emoji = validate_emoji(&body)?;

    let service = ReactionService::new(&state.db);
    let reactions = service
        .set_reaction(scoped.user_id, scoped.group_id, ticket_id, comment_id, emoji)
        .await?;
    Ok(HttpResponse::Ok().json(reactions))
}

pub async fn remove_reaction(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, comment_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let comment_id = parse_id(&comment_id)?;

    let service = ReactionService::new(&state.db);
    let reactions = service
        .remove_reaction(scoped.user_id, scoped.group_id, ticket_id, comment_id)
        .await?;
    Ok(HttpResponse::Ok().json(reactions))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(emoji: &str) -> SetReactionRequest {
        SetReactionRequest {
            emoji: emoji.to_string(),
        }
    }

    #[test]
    fn validate_emoji_rejects_blank() {
        let result = validate_emoji(&request("   "));
        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    #[test]
    fn validate_emoji_rejects_over_limit() {
        let too_long = "a".repeat(MAX_EMOJI_CHARS + 1);
        let result = validate_emoji(&request(&too_long));
        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    #[test]
    fn validate_emoji_trims_and_accepts_a_real_emoji() {
        let result = validate_emoji(&request(" \u{1F44D} ")).unwrap();
        assert_eq!(result, "\u{1F44D}");
    }
}
