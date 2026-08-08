use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::ai::models::SendChatMessageRequest;
use crate::ai::service::AiService;
use crate::errors::ApiError;
use crate::server::middleware::GroupScoped;
use crate::state::AppState;

// Same limit as comment_handlers::MAX_CONTENT_LEN — a chat message is
// free-form user text like a comment, no reason to give it a different cap.
const MAX_MESSAGE_LEN: usize = 2000;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

// .chars().count(), not .len() — same UTF-8 byte-vs-character-count reason as
// comment_handlers::validate_create.
fn validate_message(message: &str) -> Result<(), ApiError> {
    if message.trim().is_empty() {
        return Err(ApiError::Validation("message is required".to_string()));
    }
    if message.chars().count() > MAX_MESSAGE_LEN {
        return Err(ApiError::Validation(format!(
            "message must be at most {MAX_MESSAGE_LEN} characters"
        )));
    }
    Ok(())
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id (same
// pattern as ticket_handlers::get_ticket).
pub async fn summarize_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    let summary = service
        .summarize_ticket(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(summary))
}

pub async fn analyze_ticket(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    let analysis = service
        .analyze_ticket(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(analysis))
}

pub async fn generate_group_report(
    scoped: GroupScoped,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let service = AiService::new(&state.db, &state.config)?;
    let report = service
        .generate_group_report(scoped.user_id, scoped.group_id)
        .await?;
    Ok(HttpResponse::Ok().json(report))
}

pub async fn create_conversation(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    let conversation = service
        .create_conversation(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Created().json(conversation))
}

pub async fn list_conversations(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    let conversations = service
        .list_conversations(scoped.user_id, scoped.group_id, ticket_id)
        .await?;
    Ok(HttpResponse::Ok().json(conversations))
}

// GroupScoped only consumes the {id} segment, so the conversation_id segment
// (unlike ticket_id above) is a real third path element here, not something
// GroupScoped already resolved.
pub async fn send_conversation_message(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
    body: web::Json<SendChatMessageRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, conversation_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let conversation_id = parse_id(&conversation_id)?;
    let input = body.into_inner();
    validate_message(&input.message)?;

    let service = AiService::new(&state.db, &state.config)?;
    let result = service
        .send_conversation_message(
            scoped.user_id,
            scoped.group_id,
            ticket_id,
            conversation_id,
            input.message,
        )
        .await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn list_conversation_messages(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, conversation_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let conversation_id = parse_id(&conversation_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    let messages = service
        .list_conversation_messages(scoped.user_id, scoped.group_id, ticket_id, conversation_id)
        .await?;
    Ok(HttpResponse::Ok().json(messages))
}

pub async fn delete_conversation(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, ticket_id, conversation_id) = path.into_inner();
    let ticket_id = parse_id(&ticket_id)?;
    let conversation_id = parse_id(&conversation_id)?;

    let service = AiService::new(&state.db, &state.config)?;
    service
        .delete_conversation(scoped.user_id, scoped.group_id, ticket_id, conversation_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_message_rejects_empty() {
        assert!(matches!(
            validate_message("   "),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn validate_message_rejects_over_limit_by_char_count() {
        let too_long = "a".repeat(MAX_MESSAGE_LEN + 1);
        assert!(matches!(
            validate_message(&too_long),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn validate_message_accepts_within_limit() {
        assert!(validate_message("Any workaround for this?").is_ok());
    }
}
