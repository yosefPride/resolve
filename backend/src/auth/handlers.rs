use actix_web::{HttpResponse, web};

use crate::auth::models::{LoginRequest, RegisterRequest};
use crate::auth::service::AuthService;
use crate::errors::ApiError;
use crate::state::AppState;

fn validate_register(input: &RegisterRequest) -> Result<(), ApiError> {
    if input.email.trim().is_empty() || !input.email.contains('@') {
        return Err(ApiError::Validation(
            "a valid email is required".to_string(),
        ));
    }
    if input.password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".to_string(),
        ));
    }
    if input.name.trim().is_empty() {
        return Err(ApiError::Validation("name is required".to_string()));
    }
    Ok(())
}

fn validate_login(input: &LoginRequest) -> Result<(), ApiError> {
    if input.email.trim().is_empty() || input.password.is_empty() {
        return Err(ApiError::Validation(
            "email and password are required".to_string(),
        ));
    }
    Ok(())
}

pub async fn register(
    state: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_register(&input)?;

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    let response = auth_service.register(input).await?;
    Ok(HttpResponse::Created().json(response))
}

pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_login(&input)?;

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    let response = auth_service.login(input).await?;
    Ok(HttpResponse::Ok().json(response))
}
