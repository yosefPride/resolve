use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;

use crate::user::repository::UserRepoError;

#[derive(Debug)]
pub enum ApiError {
    InvalidCredentials,
    Unauthenticated,
    DuplicateEmail,
    Validation(String),
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl ApiError {
    fn code(&self) -> &'static str {
        match self {
            ApiError::InvalidCredentials => "invalid_credentials",
            ApiError::Unauthenticated => "unauthenticated",
            ApiError::DuplicateEmail => "duplicate_email",
            ApiError::Validation(_) => "validation_error",
            ApiError::Internal => "internal_error",
        }
    }

    fn message(&self) -> String {
        match self {
            ApiError::InvalidCredentials => "invalid email or password".to_string(),
            ApiError::Unauthenticated => "missing or invalid authentication token".to_string(),
            ApiError::DuplicateEmail => "email already in use".to_string(),
            ApiError::Validation(message) => message.clone(),
            ApiError::Internal => "internal server error".to_string(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            ApiError::Unauthenticated => StatusCode::UNAUTHORIZED,
            ApiError::DuplicateEmail => StatusCode::CONFLICT,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorBody {
            error: ErrorDetail {
                code: self.code().to_string(),
                message: self.message(),
            },
        })
    }
}

// Repository/service errors are never surfaced to the client directly (docs/api.md
// forbids leaking raw internal errors), so the Database variant collapses to a
// generic Internal error here.
impl From<UserRepoError> for ApiError {
    fn from(err: UserRepoError) -> Self {
        match err {
            UserRepoError::DuplicateEmail => ApiError::DuplicateEmail,
            UserRepoError::Database(_) => ApiError::Internal,
        }
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(_: bcrypt::BcryptError) -> Self {
        ApiError::Internal
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        ApiError::Internal
    }
}
