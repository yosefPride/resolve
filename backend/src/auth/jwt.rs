use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::auth::claims::Claims;

const TOKEN_TTL_HOURS: i64 = 24;

pub fn issue_token(
    user_id: &str,
    token_version: i32,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::hours(TOKEN_TTL_HOURS)).timestamp() as usize;
    issue_token_with_exp(user_id, token_version, secret, exp)
}

// Exposed (rather than a fixed TTL only) so tests can mint tokens with an
// arbitrary expiry, e.g. one already in the past, to exercise expiry handling.
pub fn issue_token_with_exp(
    user_id: &str,
    token_version: i32,
    secret: &str,
    exp: usize,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        token_version,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_token_round_trips_issue_token() {
        let token = issue_token("507f1f77bcf86cd799439011", 3, "test-secret").unwrap();
        let claims = decode_token(&token, "test-secret").unwrap();
        assert_eq!(claims.sub, "507f1f77bcf86cd799439011");
        assert_eq!(claims.token_version, 3);
    }

    #[test]
    fn decode_token_rejects_wrong_secret() {
        let token = issue_token("507f1f77bcf86cd799439011", 0, "test-secret").unwrap();
        assert!(decode_token(&token, "wrong-secret").is_err());
    }

    #[test]
    fn decode_token_rejects_expired_token() {
        let expired_exp = (Utc::now() - Duration::hours(1)).timestamp() as usize;
        let token = issue_token_with_exp("507f1f77bcf86cd799439011", 0, "test-secret", expired_exp)
            .unwrap();
        assert!(decode_token(&token, "test-secret").is_err());
    }
}
