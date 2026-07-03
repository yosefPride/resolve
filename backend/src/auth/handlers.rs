use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use actix_web::{HttpRequest, HttpResponse, web};

use crate::auth::models::{LoginRequest, RefreshResponse, RegisterRequest};
use crate::auth::refresh_token::{REFRESH_TOKEN_COOKIE, REFRESH_TOKEN_TTL_DAYS};
use crate::auth::service::AuthService;
use crate::errors::ApiError;
use crate::server::middleware::AuthenticatedUser;
use crate::state::AppState;
use crate::user::service::UserService;

// Scoped to /auth so the refresh token is never sent on unrelated API calls.
// httpOnly + Secure keep it out of reach of JS and plain-HTTP interception;
// Strict same-site means it's only ever sent from this app's own pages.
fn refresh_cookie(raw_token: String) -> Cookie<'static> {
    Cookie::build(REFRESH_TOKEN_COOKIE, raw_token)
        .path("/api/v1/auth")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::days(REFRESH_TOKEN_TTL_DAYS))
        .finish()
}

fn expired_refresh_cookie() -> Cookie<'static> {
    Cookie::build(REFRESH_TOKEN_COOKIE, "")
        .path("/api/v1/auth")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::ZERO)
        .finish()
}

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
    let (response, raw_refresh_token) = auth_service.register(input).await?;
    Ok(HttpResponse::Created()
        .cookie(refresh_cookie(raw_refresh_token))
        .json(response))
}

pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_login(&input)?;

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    let (response, raw_refresh_token) = auth_service.login(input).await?;
    Ok(HttpResponse::Ok()
        .cookie(refresh_cookie(raw_refresh_token))
        .json(response))
}

pub async fn me(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user_service = UserService::new(&state.db);
    let response = user_service
        .find_by_id(user.user_id)
        .await?
        .ok_or(ApiError::Unauthenticated)?;
    Ok(HttpResponse::Ok().json(response))
}

// Note: does not require AuthenticatedUser. The refresh token *is* the session
// identifier here — the access token may well have already expired by the
// time a client needs to refresh, so gating this on a valid access token
// would defeat the point.
pub async fn refresh(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let raw_refresh_token = req
        .cookie(REFRESH_TOKEN_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .ok_or(ApiError::Unauthenticated)?;

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    let (jwt, new_raw_refresh_token) = auth_service.refresh(&raw_refresh_token).await?;
    Ok(HttpResponse::Ok()
        .cookie(refresh_cookie(new_raw_refresh_token))
        .json(RefreshResponse { jwt }))
}

// Per-device logout: revokes only the refresh token in this request's cookie,
// so other logged-in devices are unaffected. Requires no access token either
// — an absent/already-invalid cookie is treated as already logged out.
pub async fn logout(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    if let Some(cookie) = req.cookie(REFRESH_TOKEN_COOKIE) {
        let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
        auth_service.logout(cookie.value()).await?;
    }
    Ok(HttpResponse::Ok().cookie(expired_refresh_cookie()).finish())
}
