use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

use crate::user::models::UserResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// Body for PATCH /auth/me. Both fields optional so a client can update either
// alone; `current_password` is only demanded when the email actually changes,
// since email is the login identity and the key Group Admins add members by.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMeRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub current_password: Option<String>,
}

// Body for POST /auth/me/password.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub jwt: String,
}

// Response for POST /auth/refresh. Deliberately just the access token — the
// caller already has the user from their initial login/register, and the
// rotated refresh token travels as an httpOnly cookie, never in the body.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub jwt: String,
}

// A single refresh-token session. One document per outstanding refresh token;
// `revoked_at` is set on rotation (single-use) or logout, and `expires_at` is
// backed by a TTL index so spent/expired rows are cleaned up automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub token_hash: String,
    pub created_at: BsonDateTime,
    pub expires_at: BsonDateTime,
    pub revoked_at: Option<BsonDateTime>,
}
