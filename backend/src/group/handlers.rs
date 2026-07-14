use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::group::models::{
    AddMemberRequest, CreateGroupRequest, LookupUserQuery, UpdateMemberRoleRequest,
};
use crate::group::service::GroupService;
use crate::server::middleware::{AuthenticatedUser, GroupScoped};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

fn validate_name(name: &str) -> Result<(), ApiError> {
    if name.trim().is_empty() {
        return Err(ApiError::Validation("name is required".to_string()));
    }
    Ok(())
}

pub async fn create_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    body: web::Json<CreateGroupRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_name(&input.name)?;

    let service = GroupService::new(&state.db);
    let group = service.create_group(user.user_id, input.name).await?;
    Ok(HttpResponse::Created().json(group))
}

pub async fn list_my_groups(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let service = GroupService::new(&state.db);
    let groups = service.list_my_groups(user.user_id).await?;
    Ok(HttpResponse::Ok().json(groups))
}

// The {id}-scoped handlers below take GroupScoped rather than
// AuthenticatedUser: the extractor authenticates, parses the {id} path
// segment, and verifies membership before the handler runs (docs/rbac.md,
// "Enforcement Mechanism"). The services still re-run their own role checks
// underneath — request-level enforcement is an additional layer, not a
// replacement.

pub async fn get_group(
    scoped: GroupScoped,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let service = GroupService::new(&state.db);
    let group = service.get_group(scoped.user_id, scoped.group_id).await?;
    Ok(HttpResponse::Ok().json(group))
}

pub async fn rename_group(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    body: web::Json<CreateGroupRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    validate_name(&input.name)?;

    let service = GroupService::new(&state.db);
    let group = service
        .rename_group(scoped.user_id, scoped.group_id, input.name)
        .await?;
    Ok(HttpResponse::Ok().json(group))
}

pub async fn delete_group(
    scoped: GroupScoped,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let service = GroupService::new(&state.db);
    service.delete_group(scoped.user_id, scoped.group_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn list_members(
    scoped: GroupScoped,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let service = GroupService::new(&state.db);
    let members = service.list_members(scoped.user_id, scoped.group_id).await?;
    Ok(HttpResponse::Ok().json(members))
}

pub async fn lookup_user(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    query: web::Query<LookupUserQuery>,
) -> Result<HttpResponse, ApiError> {
    let email = query.into_inner().email;
    if email.trim().is_empty() {
        return Err(ApiError::Validation("email is required".to_string()));
    }

    let service = GroupService::new(&state.db);
    let result = service
        .lookup_user_by_email(scoped.user_id, scoped.group_id, &email)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn add_member(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    body: web::Json<AddMemberRequest>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    let target_user_id = parse_id(&input.user_id)?;

    let service = GroupService::new(&state.db);
    let member = service
        .add_member(scoped.user_id, scoped.group_id, target_user_id, input.role)
        .await?;
    Ok(HttpResponse::Created().json(member))
}

// GroupScoped consumes the {id} segment; web::Path still extracts both
// segments, so the first is dropped here in favor of scoped.group_id.
pub async fn update_member_role(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateMemberRoleRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, target_user_id) = path.into_inner();
    let target_user_id = parse_id(&target_user_id)?;

    let service = GroupService::new(&state.db);
    let member = service
        .update_member_role(scoped.user_id, scoped.group_id, target_user_id, body.into_inner().role)
        .await?;
    Ok(HttpResponse::Ok().json(member))
}

// A single endpoint covers both admin-driven removal and self-service leaving:
// removing yourself doesn't require being a Group Admin (GroupService::leave_group
// has no admin check), removing someone else does (GroupService::remove_member).
// Both paths still enforce the sole-Group-Admin guard.
pub async fn remove_member(
    scoped: GroupScoped,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (_, target_user_id) = path.into_inner();
    let target_user_id = parse_id(&target_user_id)?;

    let service = GroupService::new(&state.db);
    if target_user_id == scoped.user_id {
        service.leave_group(scoped.user_id, scoped.group_id).await?;
    } else {
        service
            .remove_member(scoped.user_id, scoped.group_id, target_user_id)
            .await?;
    }
    Ok(HttpResponse::NoContent().finish())
}
