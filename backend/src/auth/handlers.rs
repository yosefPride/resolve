use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use actix_web::{HttpRequest, HttpResponse, web};

use crate::auth::models::{
    ChangePasswordRequest, LoginRequest, RefreshResponse, RegisterRequest, UpdateMeRequest,
};
use crate::auth::refresh_token::{self, REFRESH_TOKEN_COOKIE, REFRESH_TOKEN_TTL_DAYS};
use crate::auth::service::AuthService;
use crate::errors::ApiError;
use crate::server::middleware::AuthenticatedUser;
use crate::state::AppState;
use crate::user::service::UserService;

// Scoped to /auth so the refresh token is never sent on unrelated API calls.
// httpOnly keeps it out of reach of JS; `secure` is config-driven (see
// Config::cookie_secure) since a real browser refuses to store a Secure
// cookie at all over plain HTTP, which local dev runs over.
//
// SameSite=Strict is intentionally left fixed rather than made configurable:
// "site" for SameSite purposes ignores port (and, for same-registrable-domain
// hosts, ignores subdomain), so this already works for the intended
// topologies — a local dev frontend on a different port, or a production
// frontend/API split across subdomains of the same domain. It would only
// need to relax to None (+ Secure) if frontend and API ever ended up on
// genuinely unrelated domains.
fn refresh_cookie(raw_token: String, secure: bool) -> Cookie<'static> {
    Cookie::build(REFRESH_TOKEN_COOKIE, raw_token)
        .path("/api/v1/auth")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::days(REFRESH_TOKEN_TTL_DAYS))
        .finish()
}

fn expired_refresh_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build(REFRESH_TOKEN_COOKIE, "")
        .path("/api/v1/auth")
        .http_only(true)
        .secure(secure)
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

fn validate_update_me(input: &UpdateMeRequest) -> Result<(), ApiError> {
    if input.name.is_none() && input.email.is_none() {
        return Err(ApiError::Validation("nothing to update".to_string()));
    }
    if let Some(email) = &input.email {
        if email.trim().is_empty() || !email.contains('@') {
            return Err(ApiError::Validation(
                "a valid email is required".to_string(),
            ));
        }
    }
    if let Some(name) = &input.name {
        if name.trim().is_empty() {
            return Err(ApiError::Validation("name is required".to_string()));
        }
    }
    Ok(())
}

fn validate_change_password(input: &ChangePasswordRequest) -> Result<(), ApiError> {
    if input.current_password.is_empty() {
        return Err(ApiError::Validation(
            "current password is required".to_string(),
        ));
    }
    if input.new_password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".to_string(),
        ));
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
        .cookie(refresh_cookie(
            raw_refresh_token,
            state.config.cookie_secure,
        ))
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
        .cookie(refresh_cookie(
            raw_refresh_token,
            state.config.cookie_secure,
        ))
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

pub async fn update_me(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateMeRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_update_me(&input)?;

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    let response = auth_service.update_me(user.user_id, input).await?;
    Ok(HttpResponse::Ok().json(response))
}

// The request's own refresh cookie (if present) identifies the session to
// spare from revocation, so changing the password logs out every other device
// without ending the session that made the change.
pub async fn change_password(
    user: AuthenticatedUser,
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_change_password(&input)?;

    let current_token_hash = req
        .cookie(REFRESH_TOKEN_COOKIE)
        .map(|cookie| refresh_token::hash_token(cookie.value()));

    let auth_service = AuthService::new(&state.db, state.config.jwt_secret.clone());
    auth_service
        .change_password(user.user_id, input, current_token_hash.as_deref())
        .await?;
    Ok(HttpResponse::Ok().finish())
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
        .cookie(refresh_cookie(
            new_raw_refresh_token,
            state.config.cookie_secure,
        ))
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
    Ok(HttpResponse::Ok()
        .cookie(expired_refresh_cookie(state.config.cookie_secure))
        .finish())
}
