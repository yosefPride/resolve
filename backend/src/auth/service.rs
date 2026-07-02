use mongodb::Database;

use crate::auth::jwt;
use crate::auth::models::{AuthResponse, LoginRequest, RegisterRequest};
use crate::auth::password;
use crate::errors::ApiError;
use crate::user::models::CreateUserInput;
use crate::user::service::UserService;

pub struct AuthService {
    user_service: UserService,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(db: &Database, jwt_secret: String) -> Self {
        Self {
            user_service: UserService::new(db),
            jwt_secret,
        }
    }

    pub async fn register(&self, input: RegisterRequest) -> Result<AuthResponse, ApiError> {
        let password_hash = password::hash_password(&input.password)?;
        let user = self
            .user_service
            .create(CreateUserInput {
                email: input.email,
                name: input.name,
                password_hash,
            })
            .await?;

        // A newly created user always starts at token_version 0.
        let jwt = jwt::issue_token(&user.id, 0, &self.jwt_secret)?;
        Ok(AuthResponse { user, jwt })
    }

    pub async fn login(&self, input: LoginRequest) -> Result<AuthResponse, ApiError> {
        let user = self
            .user_service
            .find_by_email(&input.email)
            .await?
            .ok_or(ApiError::InvalidCredentials)?;

        let valid = password::verify_password(&input.password, &user.password_hash)?;
        if !valid {
            return Err(ApiError::InvalidCredentials);
        }

        let token_version = user.token_version;
        let user = crate::user::models::UserResponse::from(user);
        let jwt = jwt::issue_token(&user.id, token_version, &self.jwt_secret)?;
        Ok(AuthResponse { user, jwt })
    }

    pub async fn logout(&self, user_id: mongodb::bson::oid::ObjectId) -> Result<(), ApiError> {
        self.user_service.increment_token_version(user_id).await?;
        Ok(())
    }
}
