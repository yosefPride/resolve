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
