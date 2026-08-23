use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;

use crate::group::repository::GroupRepoError;
use crate::link::repository::LinkRepoError;
use crate::user::repository::UserRepoError;

#[derive(Debug)]
pub enum ApiError {
    InvalidCredentials,
    Unauthenticated,
    Forbidden,
    NotFound,
    DuplicateEmail,
    Conflict(String),
    Validation(String),
    RateLimited(String),
    // The upstream AI provider was reached but could not serve the request —
    // its free-tier quota is exhausted, or the model is overloaded. Transient
    // and worth retrying, unlike Internal. Kept distinct from RateLimited,
    // which means the caller hit *our* per-user chat quota rather than
    // Google's (see ai/client.rs's generate() for the full reasoning).
    AiUnavailable,
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
            ApiError::Forbidden => "forbidden",
            ApiError::NotFound => "not_found",
            ApiError::DuplicateEmail => "duplicate_email",
            ApiError::Conflict(_) => "conflict",
            ApiError::Validation(_) => "validation_error",
            ApiError::RateLimited(_) => "rate_limited",
            ApiError::AiUnavailable => "ai_unavailable",
            ApiError::Internal => "internal_error",
        }
    }

    fn message(&self) -> String {
        match self {
            ApiError::InvalidCredentials => "invalid email or password".to_string(),
            ApiError::Unauthenticated => "missing or invalid authentication token".to_string(),
            // Deliberately generic: doesn't distinguish "not a member" from
            // "group doesn't exist" so non-members can't enumerate group ids.
            ApiError::Forbidden => "you do not have permission to perform this action".to_string(),
            ApiError::NotFound => "resource not found".to_string(),
            ApiError::DuplicateEmail => "email already in use".to_string(),
            ApiError::Conflict(message) => message.clone(),
            ApiError::Validation(message) => message.clone(),
            ApiError::RateLimited(message) => message.clone(),
            // Deliberately doesn't name Gemini or quotas: which provider backs
            // the AI features, and why it declined, is an internal detail
            // (docs/api.md). The real status and body are logged server-side
            // in ai/client.rs instead.
            ApiError::AiUnavailable => {
                "the AI service is temporarily unavailable — please try again in a moment"
                    .to_string()
            }
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
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::DuplicateEmail => StatusCode::CONFLICT,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            ApiError::AiUnavailable => StatusCode::SERVICE_UNAVAILABLE,
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

// Also covers every repository without a domain-specific error enum — those
// return mongodb::error::Error directly (utils::RepoResult), and it collapses
// to Internal here.
impl From<mongodb::error::Error> for ApiError {
    fn from(_: mongodb::error::Error) -> Self {
        ApiError::Internal
    }
}

impl From<GroupRepoError> for ApiError {
    fn from(err: GroupRepoError) -> Self {
        match err {
            GroupRepoError::DuplicateMember => {
                ApiError::Conflict("user is already a member of this group".to_string())
            }
            GroupRepoError::Database(_) => ApiError::Internal,
        }
    }
}

impl From<LinkRepoError> for ApiError {
    fn from(err: LinkRepoError) -> Self {
        match err {
            LinkRepoError::Duplicate => ApiError::Conflict("this link already exists".to_string()),
            LinkRepoError::Database(_) => ApiError::Internal,
        }
    }
}
