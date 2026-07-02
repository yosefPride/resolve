use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use futures::future::LocalBoxFuture;
use mongodb::bson::oid::ObjectId;

use crate::auth::jwt;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::user::service::UserService;

pub struct AuthenticatedUser {
    pub user_id: ObjectId,
}

impl FromRequest for AuthenticatedUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Everything needed must be pulled out of `req` up front, as owned
        // values, since the returned future outlives this borrow of `req`.
        let header = req
            .headers()
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let state = req.app_data::<web::Data<AppState>>().cloned();

        Box::pin(async move {
            let header = header.ok_or(ApiError::Unauthenticated)?;
            let token = header
                .strip_prefix("Bearer ")
                .ok_or(ApiError::Unauthenticated)?;
            let state = state.ok_or(ApiError::Internal)?;

            let claims = jwt::decode_token(token, &state.config.jwt_secret)
                .map_err(|_| ApiError::Unauthenticated)?;
            let user_id =
                ObjectId::parse_str(&claims.sub).map_err(|_| ApiError::Unauthenticated)?;

            // The token must match the user's *current* token_version — a logout
            // bumps it, which invalidates every token issued before that point.
            let user = UserService::new(&state.db)
                .find_raw_by_id(user_id)
                .await
                .map_err(|_| ApiError::Internal)?
                .ok_or(ApiError::Unauthenticated)?;
            if user.token_version != claims.token_version {
                return Err(ApiError::Unauthenticated);
            }

            Ok(AuthenticatedUser { user_id })
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;

    use super::*;

    #[actix_web::test]
    async fn missing_authorization_header_is_rejected() {
        let req = TestRequest::default().to_http_request();
        let mut payload = Payload::None;
        let result = AuthenticatedUser::from_request(&req, &mut payload).await;
        assert!(matches!(result, Err(ApiError::Unauthenticated)));
    }

    #[actix_web::test]
    async fn non_bearer_authorization_header_is_rejected() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Basic abc123"))
            .to_http_request();
        let mut payload = Payload::None;
        let result = AuthenticatedUser::from_request(&req, &mut payload).await;
        assert!(matches!(result, Err(ApiError::Unauthenticated)));
    }
}
