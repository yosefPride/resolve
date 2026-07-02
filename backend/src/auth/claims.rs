use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,          // subject (user)
    pub exp: usize,           // expiration
    pub token_version: i32,   // must match the user's current token_version, or the token is stale
}
