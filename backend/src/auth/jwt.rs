use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::auth::claims::Claims;

// Short-lived on purpose: the access token is verified statelessly (signature +
// exp only, no DB lookup), so a stolen token's exposure window is bounded by
// this TTL rather than by an explicit revocation check. Session longevity comes
// from the refresh token instead (see auth::refresh_token).
const ACCESS_TOKEN_TTL_MINUTES: i64 = 15;

pub fn issue_token(user_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::minutes(ACCESS_TOKEN_TTL_MINUTES)).timestamp() as usize;
    issue_token_with_exp(user_id, secret, exp)
}

// Exposed (rather than a fixed TTL only) so tests can mint tokens with an
// arbitrary expiry, e.g. one already in the past, to exercise expiry handling.
pub fn issue_token_with_exp(
    user_id: &str,
    secret: &str,
    exp: usize,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
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
