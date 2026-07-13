<<<<<<< Updated upstream
=======
use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::errors::ApiError;
use crate::group::models::{
    CreateGroupInput, GroupMember, GroupResponse, MemberResponse, Role, UserLookupResponse,
};
use crate::group::repository::GroupRepository;
use crate::user::service::UserService;

pub struct GroupService {
    repo: GroupRepository,
    user_service: UserService,
}

impl GroupService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: GroupRepository::new(db),
            user_service: UserService::new(db),
        }
    }

    // Two sequential writes, not a transaction (same choice made for admin
    // user-deletion — see docs/rbac.md). If the second write fails, the group
    // is left with no members; low-probability and cheap to detect/retry
    // manually for now rather than adding session plumbing for it.
    pub async fn create_group(&self, user_id: ObjectId, name: String) -> Result<GroupResponse, ApiError> {
        let group = self
            .repo
            .create_group(CreateGroupInput {
                name,
                owner_id: user_id,
            })
            .await?;
        let group_id = group.id.expect("insert_one always returns an id");
        self.repo
            .insert_member(group_id, user_id, Role::GroupAdmin)
            .await?;
        Ok(group.into())
    }

    pub async fn list_my_groups(&self, user_id: ObjectId) -> Result<Vec<GroupResponse>, ApiError> {
        Ok(self
            .repo
            .list_groups_for_user(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn get_group(&self, user_id: ObjectId, group_id: ObjectId) -> Result<GroupResponse, ApiError> {
        self.require_member(group_id, user_id).await?;
        let group = self.repo.find_group_by_id(group_id).await?.ok_or(ApiError::NotFound)?;
        Ok(group.into())
    }

    pub async fn rename_group(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        name: String,
    ) -> Result<GroupResponse, ApiError> {
        self.require_group_admin(group_id, user_id).await?;
        self.repo.rename_group(group_id, name).await?;
        let group = self.repo.find_group_by_id(group_id).await?.ok_or(ApiError::NotFound)?;
        Ok(group.into())
    }

    pub async fn delete_group(&self, user_id: ObjectId, group_id: ObjectId) -> Result<(), ApiError> {
        self.require_group_admin(group_id, user_id).await?;
        self.repo.delete_members_by_group(group_id).await?;
        self.repo.delete_group(group_id).await?;
        Ok(())
    }

    pub async fn list_members(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<Vec<MemberResponse>, ApiError> {
        self.require_member(group_id, user_id).await?;
        let members = self.repo.list_members(group_id).await?;
        let mut result = Vec::with_capacity(members.len());
        for member in members {
            result.push(self.enrich_member(member).await?);
        }
        Ok(result)
    }

    // MemberResponse needs name/email, which GroupMember doesn't carry — this
    // is the one place that joins against the users collection to fill them
    // in. One find_by_id per member rather than a $lookup aggregation: matches
    // the rest of the repo layer (no aggregations anywhere yet), fine at
    // expected group sizes.
    async fn enrich_member(&self, member: GroupMember) -> Result<MemberResponse, ApiError> {
        let user = self.user_service.find_by_id(member.user_id).await?;
        let (name, email) = user.map(|u| (u.name, u.email)).unwrap_or_default();
        Ok(MemberResponse {
            id: member.id.map(|id| id.to_hex()).unwrap_or_default(),
            user_id: member.user_id.to_hex(),
            name,
            email,
            role: member.role,
            joined_at: DateTime::from_timestamp_millis(member.joined_at.timestamp_millis())
                .unwrap_or_default(),
        })
    }

    // Group Admin only. There is no user directory or join flow — an exact
    // email match is the only way to resolve the user_id add_member needs.
    pub async fn lookup_user_by_email(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        email: &str,
    ) -> Result<UserLookupResponse, ApiError> {
        self.require_group_admin(group_id, user_id).await?;
        let target = self
            .user_service
            .find_by_email(email)
            .await?
            .ok_or(ApiError::NotFound)?;
        Ok(UserLookupResponse {
            id: target.id.map(|id| id.to_hex()).unwrap_or_default(),
            name: target.name,
            email: target.email,
        })
    }

    pub async fn add_member(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        target_user_id: ObjectId,
        role: Role,
    ) -> Result<MemberResponse, ApiError> {
        self.require_group_admin(group_id, user_id).await?;
        let member = self.repo.insert_member(group_id, target_user_id, role).await?;
        self.enrich_member(member).await
    }

    pub async fn update_member_role(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        target_user_id: ObjectId,
        role: Role,
    ) -> Result<MemberResponse, ApiError> {
        self.require_group_admin(group_id, user_id).await?;

        if role == Role::Contributor {
            // Demoting the group's last Group Admin is blocked, same as removing them.
            self.guard_sole_admin_removal(group_id, target_user_id).await?;
        } else {
            self.repo
                .find_member(group_id, target_user_id)
                .await?
                .ok_or(ApiError::NotFound)?;
        }

        let updated = self
            .repo
            .update_member_role(group_id, target_user_id, role)
            .await?;
        if !updated {
            return Err(ApiError::NotFound);
        }

        let member = self
            .repo
            .find_member(group_id, target_user_id)
            .await?
            .ok_or(ApiError::Internal)?;
        self.enrich_member(member).await
    }

    pub async fn remove_member(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        target_user_id: ObjectId,
    ) -> Result<(), ApiError> {
        self.require_group_admin(group_id, user_id).await?;
        self.guard_sole_admin_removal(group_id, target_user_id).await?;
        let deleted = self.repo.delete_member(group_id, target_user_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }
        Ok(())
    }

    pub async fn leave_group(&self, user_id: ObjectId, group_id: ObjectId) -> Result<(), ApiError> {
        self.guard_sole_admin_removal(group_id, user_id).await?;
        let deleted = self.repo.delete_member(group_id, user_id).await?;
        if !deleted {
            return Err(ApiError::NotFound);
        }
        Ok(())
    }

    // Not a member -> Forbidden rather than NotFound, deliberately: this
    // avoids telling a non-member whether the group id even exists.
    async fn require_member(&self, group_id: ObjectId, user_id: ObjectId) -> Result<GroupMember, ApiError> {
        self.repo
            .find_member(group_id, user_id)
            .await?
            .ok_or(ApiError::Forbidden)
    }

    async fn require_group_admin(
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

    // Blocks removing/demoting a group's last Group Admin — a successor must
    // be appointed first (see docs/rbac.md, "Group Admin Succession").
    async fn guard_sole_admin_removal(
        &self,
        group_id: ObjectId,
        target_user_id: ObjectId,
    ) -> Result<(), ApiError> {
        let target = self
            .repo
            .find_member(group_id, target_user_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        if target.role == Role::GroupAdmin {
            let admin_count = self.repo.count_group_admins(group_id).await?;
            if admin_count <= 1 {
                return Err(ApiError::Conflict(
                    "a successor Group Admin must be appointed before the sole Group Admin can be removed"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}
>>>>>>> Stashed changes
