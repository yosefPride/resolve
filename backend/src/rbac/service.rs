use mongodb::{Database, bson::oid::ObjectId};

use crate::errors::ApiError;
use crate::group::models::{GroupMember, Role};
use crate::group::repository::GroupRepository;
use crate::user::models::GlobalRole;
use crate::user::service::UserService;

// Central home for the reusable RBAC primitives. These answer a single
// question — "what is this user's relationship to this group (or the system)?"
// — and carry no feature-specific logic, so tickets, comments, dashboard, and
// the group/admin modules all share them rather than growing private copies.
//
// What deliberately does NOT live here: group isolation (enforced by the
// repository-level group_id filter on every query, docs/backend.md) and the
// sole-Group-Admin succession guard (group-membership business logic that
// stays in GroupService).
pub struct RbacService {
    group_repo: GroupRepository,
    user_service: UserService,
}

impl RbacService {
    pub fn new(db: &Database) -> Self {
        Self {
            group_repo: GroupRepository::new(db),
            user_service: UserService::new(db),
        }
    }

    // Not a member -> Forbidden rather than NotFound, deliberately: this
    // avoids telling a non-member whether the group id even exists. Returns
    // the GroupMember (with its role) so a caller needing a finer decision can
    // check membership once and branch on the role instead of querying twice.
    pub async fn require_member(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<GroupMember, ApiError> {
        self.group_repo
            .find_member(group_id, user_id)
            .await?
            .ok_or(ApiError::Forbidden)
    }

    pub async fn require_group_admin(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<GroupMember, ApiError> {
        let member = self.require_member(group_id, user_id).await?;
        if member.role != Role::GroupAdmin {
            return Err(ApiError::Forbidden);
        }
        Ok(member)
    }

    // Global-scope check, independent of any group. A missing user maps to
    // Forbidden (same as a non-admin) so a stale/forged token can't probe user
    // existence.
    pub async fn require_system_admin(&self, user_id: ObjectId) -> Result<(), ApiError> {
        let caller = self
            .user_service
            .find_by_id(user_id)
            .await?
            .ok_or(ApiError::Forbidden)?;
        match caller.global_role {
            Some(GlobalRole::SystemAdmin) => Ok(()),
            _ => Err(ApiError::Forbidden),
        }
    }

    // Composite ticket/comment rule (docs/api.md, "Ticket Rules"): a
    // Contributor may act only on resources they created, a Group Admin on any
    // resource in the group. Takes an already-resolved membership (so the
    // caller does one require_member lookup and reuses it) plus the resource's
    // creator_id — pure, no DB access.
    pub fn require_owner_or_group_admin(
        member: &GroupMember,
        resource_owner_id: ObjectId,
    ) -> Result<(), ApiError> {
        if member.role == Role::GroupAdmin || member.user_id == resource_owner_id {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}
