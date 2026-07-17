use std::collections::HashMap;

use chrono::DateTime;
use mongodb::{
    Database,
    bson::{DateTime as BsonDateTime, oid::ObjectId},
};

use crate::admin::models::{
    AuditAction, AuditLogEntry, AuditLogEntryResponse, AutoDeleteGroupInfo, BlockedGroupInfo,
    DeletionCheckResponse,
};
use crate::admin::repository::AdminRepository;
use crate::errors::ApiError;
use crate::group::models::{GroupMember, GroupResponse, MemberResponse, Role};
use crate::group::repository::GroupRepository;
use crate::rbac::service::RbacService;
use crate::user::models::UserResponse;
use crate::user::service::UserService;

#[derive(Default)]
struct DeletionPlan {
    // Sole Group Admin, other members exist — needs an explicit successor.
    blocked: Vec<(ObjectId, String, Vec<GroupMember>)>,
    // Sole Group Admin, no other members — group is deleted outright.
    auto_delete: Vec<(ObjectId, String)>,
    // Contributor, or Group Admin alongside other admins — plain membership removal.
    plain_removals: Vec<ObjectId>,
}

pub struct AdminService {
    group_repo: GroupRepository,
    user_service: UserService,
    admin_repo: AdminRepository,
    rbac: RbacService,
}

impl AdminService {
    pub fn new(db: &Database) -> Self {
        Self {
            group_repo: GroupRepository::new(db),
            user_service: UserService::new(db),
            admin_repo: AdminRepository::new(db),
            rbac: RbacService::new(db),
        }
    }

    pub async fn deletion_check(
        &self,
        caller_id: ObjectId,
        target_user_id: ObjectId,
    ) -> Result<DeletionCheckResponse, ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        self.user_service
            .find_by_id(target_user_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let plan = self.build_plan(target_user_id).await?;

        let mut blocked_groups = Vec::with_capacity(plan.blocked.len());
        for (group_id, group_name, others) in plan.blocked {
            blocked_groups.push(BlockedGroupInfo {
                group_id: group_id.to_hex(),
                group_name,
                eligible_successors: self.enrich_members(others).await?,
            });
        }

        Ok(DeletionCheckResponse {
            blocked_groups,
            auto_delete_groups: plan
                .auto_delete
                .into_iter()
                .map(|(group_id, group_name)| AutoDeleteGroupInfo {
                    group_id: group_id.to_hex(),
                    group_name,
                })
                .collect(),
        })
    }

    // Re-derives the plan itself rather than trusting a client-supplied one —
    // group membership may have changed since the caller last called
    // deletion_check. Validates every blocked group has a valid successor
    // *before* performing any writes, then executes sequentially (not a Mongo
    // transaction, see docs/rbac.md), with the user document deleted last so
    // a mid-failure retry is always safe.
    pub async fn delete_user(
        &self,
        caller_id: ObjectId,
        target_user_id: ObjectId,
        successors: HashMap<ObjectId, ObjectId>,
    ) -> Result<(), ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        let target_user = self
            .user_service
            .find_by_id(target_user_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        // Snapshot names for the audit log now, while the entities still exist.
        let deleted_user_name = target_user.name;
        let performed_by_name = self
            .user_service
            .find_by_id(caller_id)
            .await?
            .map(|u| u.name)
            .unwrap_or_default();

        let plan = self.build_plan(target_user_id).await?;

        for (group_id, _name, others) in &plan.blocked {
            let successor_id = successors.get(group_id).ok_or_else(|| {
                ApiError::Conflict(format!(
                    "a successor is required for group {}",
                    group_id.to_hex()
                ))
            })?;
            if !others.iter().any(|m| &m.user_id == successor_id) {
                return Err(ApiError::Conflict(format!(
                    "successor is not a member of group {}",
                    group_id.to_hex()
                )));
            }
        }

        for (group_id, group_name, _others) in &plan.blocked {
            let successor_id = successors[group_id];
            let successor_name = self
                .user_service
                .find_by_id(successor_id)
                .await?
                .map(|u| u.name)
                .unwrap_or_default();
            self.group_repo
                .update_member_role(*group_id, successor_id, Role::GroupAdmin)
                .await?;
            self.group_repo
                .delete_member(*group_id, target_user_id)
                .await?;
            self.admin_repo
                .insert_audit_entry(AuditLogEntry {
                    id: None,
                    action: AuditAction::Succession,
                    group_id: *group_id,
                    group_name: group_name.clone(),
                    deleted_user_id: target_user_id,
                    deleted_user_name: deleted_user_name.clone(),
                    successor_user_id: Some(successor_id),
                    successor_user_name: Some(successor_name),
                    performed_by: caller_id,
                    performed_by_name: performed_by_name.clone(),
                    created_at: BsonDateTime::now(),
                })
                .await?;
        }

        for (group_id, group_name) in &plan.auto_delete {
            self.group_repo.delete_members_by_group(*group_id).await?;
            self.group_repo.delete_group(*group_id).await?;
            self.admin_repo
                .insert_audit_entry(AuditLogEntry {
                    id: None,
                    action: AuditAction::GroupAutoDeleted,
                    group_id: *group_id,
                    group_name: group_name.clone(),
                    deleted_user_id: target_user_id,
                    deleted_user_name: deleted_user_name.clone(),
                    successor_user_id: None,
                    successor_user_name: None,
                    performed_by: caller_id,
                    performed_by_name: performed_by_name.clone(),
                    created_at: BsonDateTime::now(),
                })
                .await?;
        }

        for group_id in &plan.plain_removals {
            self.group_repo
                .delete_member(*group_id, target_user_id)
                .await?;
        }

        self.user_service.delete(target_user_id).await?;

        Ok(())
    }

    pub async fn list_users(&self, caller_id: ObjectId) -> Result<Vec<UserResponse>, ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        Ok(self.user_service.list_all().await?)
    }

    // Read-only view of the succession/auto-deletion audit trail, System Admin
    // only. Filters are optional and independent; results are newest-first.
    pub async fn list_audit_log(
        &self,
        caller_id: ObjectId,
        group_id: Option<ObjectId>,
        deleted_user_id: Option<ObjectId>,
    ) -> Result<Vec<AuditLogEntryResponse>, ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        Ok(self
            .admin_repo
            .list_audit_log(group_id, deleted_user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn list_groups(&self, caller_id: ObjectId) -> Result<Vec<GroupResponse>, ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        Ok(self
            .group_repo
            .list_all_groups(None)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    // No membership or succession check — unlike delete_user, deleting the
    // whole group removes the "at least one Group Admin" invariant along with
    // it, so there's no one left to preserve continuity for. Group Admins
    // deleting their own group already go through GroupService::delete_group
    // instead; this is the System-Admin-as-non-member path.
    pub async fn delete_group(&self, caller_id: ObjectId, group_id: ObjectId) -> Result<(), ApiError> {
        self.rbac.require_system_admin(caller_id).await?;
        self.group_repo.delete_members_by_group(group_id).await?;
        let deleted = self.group_repo.delete_group(group_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }
        Ok(())
    }

    // Same enrichment GroupService::enrich_member does (MemberResponse needs
    // name/email, which GroupMember doesn't carry) — duplicated rather than
    // shared, since AdminService already holds its own UserService and this
    // is the only place it needs it.
    async fn enrich_members(&self, members: Vec<GroupMember>) -> Result<Vec<MemberResponse>, ApiError> {
        let mut result = Vec::with_capacity(members.len());
        for member in members {
            let user = self.user_service.find_by_id(member.user_id).await?;
            let (name, email) = user.map(|u| (u.name, u.email)).unwrap_or_default();
            result.push(MemberResponse {
                id: member.id.map(|id| id.to_hex()).unwrap_or_default(),
                user_id: member.user_id.to_hex(),
                name,
                email,
                role: member.role,
                joined_at: DateTime::from_timestamp_millis(member.joined_at.timestamp_millis())
                    .unwrap_or_default(),
            });
        }
        Ok(result)
    }

    // Walks every group the target belongs to and classifies each one. Shared
    // by deletion_check (preview) and delete_user (re-validated at commit time).
    async fn build_plan(&self, target_user_id: ObjectId) -> Result<DeletionPlan, ApiError> {
        let groups = self.group_repo.list_groups_for_user(target_user_id).await?;
        let mut plan = DeletionPlan::default();

        for group in groups {
            let group_id = group.id.expect("listed groups always have an id");
            let membership = self
                .group_repo
                .find_member(group_id, target_user_id)
                .await?
                .ok_or(ApiError::Internal)?;

            if membership.role != Role::GroupAdmin {
                plan.plain_removals.push(group_id);
                continue;
            }

            let admin_count = self.group_repo.count_group_admins(group_id).await?;
            if admin_count > 1 {
                plan.plain_removals.push(group_id);
                continue;
            }

            let members = self.group_repo.list_members(group_id).await?;
            let others: Vec<GroupMember> = members
                .into_iter()
                .filter(|m| m.user_id != target_user_id)
                .collect();

            if others.is_empty() {
                plan.auto_delete.push((group_id, group.name));
            } else {
                plan.blocked.push((group_id, group.name, others));
            }
        }

        Ok(plan)
    }
}
