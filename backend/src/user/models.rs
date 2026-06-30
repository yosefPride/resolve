use chrono::{DateTime, Utc};
use mongodb::bson::{oid::ObjectId, DateTime as BsonDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GlobalRole {
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub global_role: Option<GlobalRole>,
    pub created_at: BsonDateTime,
}

pub struct CreateUserInput {
    pub email: String,
    pub name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub global_role: Option<GlobalRole>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id.map(|id| id.to_hex()).unwrap_or_default(),
            email: user.email,
            name: user.name,
            global_role: user.global_role,
            created_at: DateTime::from_timestamp_millis(user.created_at.timestamp_millis())
                .unwrap_or_default(),
        }
    }
}
