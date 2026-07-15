use std::collections::HashMap;

use actix_web::{HttpResponse, web};
use mongodb::bson::oid::ObjectId;

use crate::admin::models::{AuditLogQuery, DeleteUserRequest};
use crate::admin::service::AdminService;
use crate::errors::ApiError;
use crate::server::middleware::SystemAdminUser;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(raw).map_err(|_| ApiError::Validation("invalid id".to_string()))
}

pub async fn deletion_check(
    user: SystemAdminUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let target_id = parse_id(&path.into_inner())?;
    let service = AdminService::new(&state.db);
    let result = service.deletion_check(user.user_id, target_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete_user(
    user: SystemAdminUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<DeleteUserRequest>,
) -> Result<HttpResponse, ApiError> {
    let target_id = parse_id(&path.into_inner())?;

    let mut successors = HashMap::new();
    for (group_id, successor_id) in body.into_inner().successors {
        successors.insert(parse_id(&group_id)?, parse_id(&successor_id)?);
    }

    let service = AdminService::new(&state.db);
    service.delete_user(user.user_id, target_id, successors).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn list_users(user: SystemAdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let service = AdminService::new(&state.db);
    let users = service.list_users(user.user_id).await?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn list_groups(user: SystemAdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let service = AdminService::new(&state.db);
    let groups = service.list_groups(user.user_id).await?;
    Ok(HttpResponse::Ok().json(groups))
}

pub async fn list_audit_log(
    user: SystemAdminUser,
    state: web::Data<AppState>,
    query: web::Query<AuditLogQuery>,
) -> Result<HttpResponse, ApiError> {
    let query = query.into_inner();
    let group_id = query.group_id.as_deref().map(parse_id).transpose()?;
    let deleted_user_id = query.user_id.as_deref().map(parse_id).transpose()?;

    let service = AdminService::new(&state.db);
    let entries = service
        .list_audit_log(user.user_id, group_id, deleted_user_id)
        .await?;
    Ok(HttpResponse::Ok().json(entries))
}

pub async fn delete_group(
    user: SystemAdminUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_id = parse_id(&path.into_inner())?;
    let service = AdminService::new(&state.db);
    service.delete_group(user.user_id, group_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
