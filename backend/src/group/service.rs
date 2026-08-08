use chrono::DateTime;
use mongodb::{Database, bson::oid::ObjectId};

use crate::activity::repository::ActivityRepository;
use crate::ai::repository::AiRepository;
use crate::comment::repository::CommentRepository;
use crate::errors::ApiError;
use crate::link::repository::LinkRepository;
use crate::reference::repository::ReferenceRepository;
use crate::group::models::{
    CreateGroupInput, GroupMember, GroupResponse, GroupSummaryResponse, MemberResponse, Role,
    UserLookupResponse,
};
use crate::group::repository::GroupRepository;
use crate::rbac::service::RbacService;
use crate::ticket::repository::TicketRepository;
use crate::user::service::UserService;

// The one place that knows what "a group's data" consists of. Every path that
// destroys a group goes through here — GroupService::delete_group (Group Admin
// deleting their own), AdminService::delete_group (System Admin deleting any),
// and AdminService::delete_user (sole-admin auto-deletion). Each of those keeps
// its own authorization check; only the cascade is shared, so a newly added
// per-group collection is wired in once here instead of three times.
//
// Deletion order is child-to-parent: the group document goes last, so a failure
// partway through leaves the group still resolvable and the cascade re-runnable
// rather than orphaning what it was supposed to remove. Sequential writes, not
// a transaction — the same choice made in create_group and admin user-deletion.
//
// comment_repo.delete_by_group unconditionally hard-deletes every comment
// (tombstones included) for the group — comments carry their own group_id, so
// no per-ticket fan-out is needed, and there's nothing left for a reply to stay
// valid against once the whole group is gone. Deleting a *single* ticket is a
// separate, smaller concern and does NOT belong here — that cascade lives in
// TicketService::delete_ticket via CommentRepository::delete_by_ticket, or it
// would take the whole group down with one ticket.
pub async fn purge_group_data(
    repo: &GroupRepository,
    ticket_repo: &TicketRepository,
    comment_repo: &CommentRepository,
    ai_repo: &AiRepository,
    activity_repo: &ActivityRepository,
    link_repo: &LinkRepository,
    reference_repo: &ReferenceRepository,
    group_id: ObjectId,
) -> Result<bool, ApiError> {
    repo.delete_members_by_group(group_id).await?;
    ticket_repo.delete_by_group(group_id).await?;
    comment_repo.delete_by_group(group_id).await?;
    ai_repo.delete_by_group(group_id).await?;
    activity_repo.delete_by_group(group_id).await?;
    link_repo.delete_by_group(group_id).await?;
    reference_repo.delete_by_group(group_id).await?;
    Ok(repo.delete_group(group_id).await?)
}

pub struct GroupService {
    repo: GroupRepository,
    ticket_repo: TicketRepository,
    comment_repo: CommentRepository,
    ai_repo: AiRepository,
    activity_repo: ActivityRepository,
    link_repo: LinkRepository,
    reference_repo: ReferenceRepository,
    user_service: UserService,
    rbac: RbacService,
}

impl GroupService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: GroupRepository::new(db),
            ticket_repo: TicketRepository::new(db),
            comment_repo: CommentRepository::new(db),
            ai_repo: AiRepository::new(db),
            activity_repo: ActivityRepository::new(db),
            link_repo: LinkRepository::new(db),
            reference_repo: ReferenceRepository::new(db),
            user_service: UserService::new(db),
            rbac: RbacService::new(db),
        }
    }

    // Two sequential writes, not a transaction (same choice made for admin
    // user-deletion — see docs/rbac.md). If the second write fails, the group
    // is left with no members; low-probability and cheap to detect/retry
    // manually for now rather than adding session plumbing for it.
    pub async fn create_group(
        &self,
        user_id: ObjectId,
        name: String,
    ) -> Result<GroupResponse, ApiError> {
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

    pub async fn list_my_groups(&self, user_id: ObjectId) -> Result<Vec<GroupSummaryResponse>, ApiError> {
        let memberships = self.repo.list_memberships_for_user(user_id).await?;
        let mut result = Vec::with_capacity(memberships.len());
        for membership in memberships {
            let group = self
                .repo
                .find_group_by_id(membership.group_id)
                .await?
                .ok_or(ApiError::Internal)?;
            let member_count = self.repo.count_members(membership.group_id).await?;
            let open_ticket_count = self
                .ticket_repo
                .count_open_by_group(membership.group_id)
                .await?;
            result.push(GroupSummaryResponse {
                id: group.id.map(|id| id.to_hex()).unwrap_or_default(),
                name: group.name,
                role: membership.role,
                member_count,
                open_ticket_count,
                created_at: DateTime::from_timestamp_millis(group.created_at.timestamp_millis())
                    .unwrap_or_default(),
            });
        }
        Ok(result)
    }

    pub async fn get_group(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<GroupResponse, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
        let group = self
            .repo
            .find_group_by_id(group_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Ok(group.into())
    }

    pub async fn rename_group(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        name: String,
    ) -> Result<GroupResponse, ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;
        self.repo.rename_group(group_id, name).await?;
        let group = self
            .repo
            .find_group_by_id(group_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Ok(group.into())
    }

    pub async fn delete_group(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<(), ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;
        purge_group_data(
            &self.repo,
            &self.ticket_repo,
            &self.comment_repo,
            &self.ai_repo,
            &self.activity_repo,
            &self.link_repo,
            &self.reference_repo,
            group_id,
        )
        .await?;
        Ok(())
    }

    pub async fn list_members(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
    ) -> Result<Vec<MemberResponse>, ApiError> {
        self.rbac.require_member(group_id, user_id).await?;
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
        self.rbac.require_group_admin(group_id, user_id).await?;
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
        self.rbac.require_group_admin(group_id, user_id).await?;
        let member = self
            .repo
            .insert_member(group_id, target_user_id, role)
            .await?;
        self.enrich_member(member).await
    }

    pub async fn update_member_role(
        &self,
        user_id: ObjectId,
        group_id: ObjectId,
        target_user_id: ObjectId,
        role: Role,
    ) -> Result<MemberResponse, ApiError> {
        self.rbac.require_group_admin(group_id, user_id).await?;

        if role == Role::Contributor {
            // Demoting the group's last Group Admin is blocked, same as removing them.
            self.guard_sole_admin_removal(group_id, target_user_id)
                .await?;
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
        self.rbac.require_group_admin(group_id, user_id).await?;
        self.guard_sole_admin_removal(group_id, target_user_id)
            .await?;
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
