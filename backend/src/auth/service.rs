use chrono::{Duration, Utc};
use mongodb::Database;
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};

use crate::auth::jwt;
use crate::auth::models::{AuthResponse, LoginRequest, RegisterRequest, UpdateMeRequest};
use crate::auth::password;
use crate::auth::refresh_token::{self, REFRESH_TOKEN_TTL_DAYS};
use crate::auth::repository::AuthRepository;
use crate::errors::ApiError;
use crate::user::models::CreateUserInput;
use crate::user::service::UserService;

pub struct AuthService {
    user_service: UserService,
    auth_repo: AuthRepository,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(db: &Database, jwt_secret: String) -> Self {
        Self {
            user_service: UserService::new(db),
            auth_repo: AuthRepository::new(db),
            jwt_secret,
        }
    }

    // Mints a fresh session for a user: a short-lived access token plus a new
    // refresh-token row. Shared by register, login, and refresh (rotation) so
    // all three issue sessions the exact same way.
    async fn issue_session(&self, user_id: &str) -> Result<(String, String), ApiError> {
        let jwt = jwt::issue_token(user_id, &self.jwt_secret)?;

        let (raw_refresh_token, token_hash) = refresh_token::generate();
        let expires_at = BsonDateTime::from_millis(
            (Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS)).timestamp_millis(),
        );
        let user_object_id = ObjectId::parse_str(user_id).map_err(|_| ApiError::Internal)?;
        self.auth_repo
            .insert(user_object_id, token_hash, expires_at)
            .await?;

        Ok((jwt, raw_refresh_token))
    }

    /// Returns the JSON body (user + access token) alongside the raw refresh
    /// token, which the handler sets as an httpOnly cookie rather than
    /// returning in the body.
    pub async fn register(
        &self,
        input: RegisterRequest,
    ) -> Result<(AuthResponse, String), ApiError> {
        let password_hash = password::hash_password(&input.password)?;
        let user = self
            .user_service
            .create(CreateUserInput {
                email: input.email,
                name: input.name,
                password_hash,
            })
            .await?;

        let (jwt, raw_refresh_token) = self.issue_session(&user.id).await?;
        Ok((AuthResponse { user, jwt }, raw_refresh_token))
    }

    pub async fn login(&self, input: LoginRequest) -> Result<(AuthResponse, String), ApiError> {
        let user = self
            .user_service
            .find_by_email(&input.email)
            .await?
            .ok_or(ApiError::InvalidCredentials)?;

        let valid = password::verify_password(&input.password, &user.password_hash)?;
        if !valid {
            return Err(ApiError::InvalidCredentials);
        }

        let user = crate::user::models::UserResponse::from(user);
        let (jwt, raw_refresh_token) = self.issue_session(&user.id).await?;
        Ok((AuthResponse { user, jwt }, raw_refresh_token))
    }

    /// Updates the caller's own name/email. Changing the email requires the
    /// current password (verified against the stored hash); a name-only change
    /// does not. Lives in AuthService rather than UserService because of that
    /// password check.
    pub async fn update_me(
        &self,
        user_id: ObjectId,
        input: UpdateMeRequest,
    ) -> Result<crate::user::models::UserResponse, ApiError> {
        let user = self
            .user_service
            .find_full_by_id(user_id)
            .await?
            .ok_or(ApiError::Unauthenticated)?;

        let name = input.name.unwrap_or_else(|| user.name.clone());
        let email = input.email.unwrap_or_else(|| user.email.clone());

        if email != user.email {
            let current_password = input.current_password.as_deref().ok_or_else(|| {
                ApiError::Validation(
                    "current password is required to change email".to_string(),
                )
            })?;
            let valid = password::verify_password(current_password, &user.password_hash)?;
            if !valid {
                return Err(ApiError::InvalidCredentials);
            }
        }

        self.user_service
            .update_profile(user_id, name.trim(), email.trim())
            .await?
            .ok_or(ApiError::Unauthenticated)
    }

    /// Exchanges a valid, unexpired, not-yet-used refresh token for a new
    /// session. The presented token is revoked first (single-use rotation) —
    /// a stolen copy of it stops working the moment the legitimate client
    /// refreshes, even without any cross-session reuse tracking.
    pub async fn refresh(&self, raw_refresh_token: &str) -> Result<(String, String), ApiError> {
        let token_hash = refresh_token::hash_token(raw_refresh_token);
        let record = self
            .auth_repo
            .find_active_by_hash(&token_hash)
            .await?
            .ok_or(ApiError::Unauthenticated)?;

        let record_id = record.id.expect("persisted refresh token always has an id");
        self.auth_repo.revoke_by_id(record_id).await?;

        self.issue_session(&record.user_id.to_hex()).await
    }

    /// Revokes a single session's refresh token — logout is per-device, not
    /// global. A missing/unknown token is treated as a no-op rather than an
    /// error, since the end state ("this token no longer works") already holds.
    pub async fn logout(&self, raw_refresh_token: &str) -> Result<(), ApiError> {
        let token_hash = refresh_token::hash_token(raw_refresh_token);
        self.auth_repo.revoke_by_hash(&token_hash).await?;
        Ok(())
    }
}
