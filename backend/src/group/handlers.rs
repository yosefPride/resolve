use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::errors::ApiError;
use crate::group::models::{AddMemberRequest, CreateGroupRequest, UpdateMemberRoleRequest};
use crate::group::service::GroupService;
use crate::server::middleware::AuthenticatedUser;
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

pub async fn get_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let service = GroupService::new(&state.db);
    let group = service.get_group(user.user_id, group_id).await?;
    Ok(HttpResponse::Ok().json(group))
}

pub async fn rename_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateGroupRequest>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let input = body.into_inner();
    validate_name(&input.name)?;

    let service = GroupService::new(&state.db);
    let group = service
        .rename_group(user.user_id, group_id, input.name)
        .await?;
    Ok(HttpResponse::Ok().json(group))
}

pub async fn delete_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let service = GroupService::new(&state.db);
    service.delete_group(user.user_id, group_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn list_members(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let service = GroupService::new(&state.db);
    let members = service.list_members(user.user_id, group_id).await?;
    Ok(HttpResponse::Ok().json(members))
}

pub async fn add_member(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<AddMemberRequest>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let input = body.into_inner();
    let target_user_id = parse_id(&input.user_id)?;

    let service = GroupService::new(&state.db);
    let member = service
        .add_member(user.user_id, group_id, target_user_id, input.role)
        .await?;
    Ok(HttpResponse::Created().json(member))
}

pub async fn update_member_role(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateMemberRoleRequest>,
) -> Result<HttpResponse, ApiError> {
    let (group_id, target_user_id) = path.into_inner();
    let group_id = parse_id(&group_id)?;
    let target_user_id = parse_id(&target_user_id)?;

    let service = GroupService::new(&state.db);
    let member = service
        .update_member_role(user.user_id, group_id, target_user_id, body.into_inner().role)
        .await?;
    Ok(HttpResponse::Ok().json(member))
}

// A single endpoint covers both admin-driven removal and self-service leaving:
// removing yourself doesn't require being a Group Admin (GroupService::leave_group
// has no admin check), removing someone else does (GroupService::remove_member).
// Both paths still enforce the sole-Group-Admin guard.
pub async fn remove_member(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (group_id, target_user_id) = path.into_inner();
    let group_id = parse_id(&group_id)?;
    let target_user_id = parse_id(&target_user_id)?;

    let service = GroupService::new(&state.db);
    if target_user_id == user.user_id {
        service.leave_group(user.user_id, group_id).await?;
    } else {
        service
            .remove_member(user.user_id, group_id, target_user_id)
            .await?;
    }
    Ok(HttpResponse::NoContent().finish())
}
