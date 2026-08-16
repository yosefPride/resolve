use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;

use crate::activity::repository::ActivityRepoError;
use crate::admin::repository::AdminRepoError;
use crate::ai::repository::AiRepoError;
use crate::comment::repository::CommentRepoError;
use crate::group::repository::GroupRepoError;
use crate::link::repository::LinkRepoError;
use crate::reaction::repository::ReactionRepoError;
use crate::reference::repository::ReferenceRepoError;
use crate::ticket::repository::TicketRepoError;
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

impl From<AdminRepoError> for ApiError {
    fn from(_: AdminRepoError) -> Self {
        ApiError::Internal
    }
}

impl From<TicketRepoError> for ApiError {
    fn from(_: TicketRepoError) -> Self {
        ApiError::Internal
    }
}

impl From<CommentRepoError> for ApiError {
    fn from(_: CommentRepoError) -> Self {
        ApiError::Internal
    }
}

impl From<AiRepoError> for ApiError {
    fn from(_: AiRepoError) -> Self {
        ApiError::Internal
    }
}

impl From<ActivityRepoError> for ApiError {
    fn from(_: ActivityRepoError) -> Self {
        ApiError::Internal
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

impl From<ReferenceRepoError> for ApiError {
    fn from(_: ReferenceRepoError) -> Self {
        ApiError::Internal
    }
}

impl From<ReactionRepoError> for ApiError {
    fn from(_: ReactionRepoError) -> Self {
        ApiError::Internal
    }
}

