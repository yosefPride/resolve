use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use futures::future::LocalBoxFuture;
use mongodb::bson::oid::ObjectId;

use crate::auth::jwt;
use crate::errors::ApiError;
use crate::group::models::Role;
use crate::rbac::service::RbacService;
use crate::state::AppState;

// --- shared helpers ---

fn authorization_header(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

// Header check is kept separate from (and ahead of) any AppState access so the
// "no/!Bearer token" paths resolve to Unauthenticated without needing state —
// which is also what lets the extractors be unit-tested without a live DB.
fn bearer_token(header: Option<String>) -> Result<String, ApiError> {
    let header = header.ok_or(ApiError::Unauthenticated)?;
    header
        .strip_prefix("Bearer ")
        .map(str::to_string)
        .ok_or(ApiError::Unauthenticated)
}

fn user_id_from_token(token: &str, secret: &str) -> Result<ObjectId, ApiError> {
    let claims = jwt::decode_token(token, secret).map_err(|_| ApiError::Unauthenticated)?;
    ObjectId::parse_str(&claims.sub).map_err(|_| ApiError::Unauthenticated)
}

// --- AuthenticatedUser ---

pub struct AuthenticatedUser {
    pub user_id: ObjectId,
}

// Fully stateless: verified by signature + exp alone, no DB lookup. This is
// safe specifically because access tokens are short-lived (see
// auth::jwt::ACCESS_TOKEN_TTL_MINUTES) — revocation happens at the refresh
// token layer, and a stolen access token simply expires on its own shortly
// after. Session-level revocation is the refresh token's job (see
// auth::repository::AuthRepository), not this extractor's.
//
// Use this on routes where authentication alone is the requirement (auth
// endpoints, cross-group group management). Data routes scoped to one tenant
// should take GroupScoped instead, and system routes SystemAdminUser.
impl FromRequest for AuthenticatedUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Everything needed must be pulled out of `req` up front, as owned
        // values, since the returned future outlives this borrow of `req`.
        let header = authorization_header(req);
        let state = req.app_data::<web::Data<AppState>>().cloned();

        Box::pin(async move {
            let token = bearer_token(header)?;
            let state = state.ok_or(ApiError::Internal)?;
            let user_id = user_id_from_token(&token, &state.config.jwt_secret)?;
            Ok(AuthenticatedUser { user_id })
        })
    }
}

// --- GroupScoped ---

// The tenant-scoped request context: caller identity plus their live role in
// the group named by the `{id}` path segment. One membership lookup happens
// here (via RbacService::require_member) so the role is always current — a user
// removed or demoted mid-token is rejected on their very next request rather
// than at token expiry. Handlers scoped to a group take this and never parse a
// group id themselves; repositories receive `group_id` from it.
//
// Service-layer role checks still run underneath (docs/rbac.md mandates both
// layers) — this extractor is the request-level half, not a replacement.
pub struct GroupScoped {
    pub user_id: ObjectId,
    pub group_id: ObjectId,
    pub role: Role,
}

impl FromRequest for GroupScoped {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = authorization_header(req);
        let state = req.app_data::<web::Data<AppState>>().cloned();
        // The group id lives in the `{id}` path segment by convention (e.g.
        // /groups/{id}/tickets). Absent segment => the extractor was mounted on
        // a route without one, which is a server misconfiguration (Internal),
        // distinct from a malformed id supplied by the client (Validation).
        let group_id_raw = req.match_info().get("id").map(str::to_string);

        Box::pin(async move {
            let token = bearer_token(header)?;
            let state = state.ok_or(ApiError::Internal)?;
            let user_id = user_id_from_token(&token, &state.config.jwt_secret)?;

            let group_id_raw = group_id_raw.ok_or(ApiError::Internal)?;
            let group_id = ObjectId::parse_str(&group_id_raw)
                .map_err(|_| ApiError::Validation("invalid id".to_string()))?;

            let member = RbacService::new(&state.db)
                .require_member(group_id, user_id)
                .await?;

            Ok(GroupScoped {
                user_id,
                group_id,
                role: member.role,
            })
        })
    }
}

// --- SystemAdminUser ---

// Global-scope guard for /admin routes: authenticates, then confirms the caller
// holds the System Admin global role (DB lookup via RbacService). A non-admin
// or unknown caller resolves to Forbidden. Service-layer require_system_admin
// still runs underneath.
pub struct SystemAdminUser {
    pub user_id: ObjectId,
}

impl FromRequest for SystemAdminUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = authorization_header(req);
        let state = req.app_data::<web::Data<AppState>>().cloned();

        Box::pin(async move {
            let token = bearer_token(header)?;
            let state = state.ok_or(ApiError::Internal)?;
            let user_id = user_id_from_token(&token, &state.config.jwt_secret)?;

            RbacService::new(&state.db)
                .require_system_admin(user_id)
                .await?;

            Ok(SystemAdminUser { user_id })
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;

    use super::*;

    // AuthenticatedUser, GroupScoped, and SystemAdminUser share the same
    // header-first handling, so the no/!Bearer cases are covered once per
    // extractor at the header layer — none of these reach the database.

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

    #[actix_web::test]
    async fn group_scoped_missing_header_is_rejected() {
        let req = TestRequest::default().to_http_request();
        let mut payload = Payload::None;
        let result = GroupScoped::from_request(&req, &mut payload).await;
        assert!(matches!(result, Err(ApiError::Unauthenticated)));
    }

    #[actix_web::test]
    async fn system_admin_missing_header_is_rejected() {
        let req = TestRequest::default().to_http_request();
        let mut payload = Payload::None;
        let result = SystemAdminUser::from_request(&req, &mut payload).await;
        assert!(matches!(result, Err(ApiError::Unauthenticated)));
    }
}
