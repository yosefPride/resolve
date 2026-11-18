import { ApiError } from '../errors.js';
import { Role, groupResponse, memberResponse } from '../group/models.js';
import { GroupRepository } from '../group/repository.js';
import { purgeGroupData } from '../group/service.js';
import { RbacService } from '../rbac/service.js';
import { GlobalRole } from '../user/models.js';
import { UserService } from '../user/service.js';
import { AuditAction, auditEntry, auditEntryResponse } from './models.js';
import { AdminRepository } from './repository.js';

export class AdminService {
  constructor(db) {
    this.db = db;
    this.groupRepo = new GroupRepository(db);
    this.userService = new UserService(db);
    this.adminRepo = new AdminRepository(db);
    this.rbac = new RbacService(db);
  }

  async deletionCheck(callerId, targetUserId) {
    await this.rbac.requireSystemAdmin(callerId);
    if (!(await this.userService.findById(targetUserId))) throw ApiError.notFound();

    const plan = await this.buildPlan(targetUserId);

    const blockedGroups = [];
    for (const { groupId, groupName, others } of plan.blocked) {
      blockedGroups.push({
        group_id: groupId.toHexString(),
        group_name: groupName,
        eligible_successors: await this.enrichMembers(others),
      });
    }

    return {
      blocked_groups: blockedGroups,
      auto_delete_groups: plan.autoDelete.map(({ groupId, groupName }) => ({
        group_id: groupId.toHexString(),
        group_name: groupName,
      })),
    };
  }

  // Re-derives the plan rather than trusting a client-supplied one —
  // membership may have changed since the caller's deletion check. Validates
  // every blocked group has a valid successor *before* any write, then runs
  // sequentially (not a transaction), deleting the user document last so a
  // mid-failure retry is always safe. `successors` maps group id hex -> id.
  async deleteUser(callerId, targetUserId, successors) {
    await this.rbac.requireSystemAdmin(callerId);
    const targetUser = await this.userService.findById(targetUserId);
    if (!targetUser) throw ApiError.notFound();
    // Snapshot names for the audit log now, while the entities still exist.
    const deletedUserName = targetUser.name;
    const performedByName = (await this.userService.findById(callerId))?.name ?? '';

    const plan = await this.buildPlan(targetUserId);

    for (const { groupId, others } of plan.blocked) {
      const successorId = successors.get(groupId.toHexString());
      if (!successorId) {
        throw ApiError.conflict(`a successor is required for group ${groupId.toHexString()}`);
      }
      if (!others.some((m) => m.user_id.equals(successorId))) {
        throw ApiError.conflict(`successor is not a member of group ${groupId.toHexString()}`);
      }
    }

    for (const { groupId, groupName } of plan.blocked) {
      const successorId = successors.get(groupId.toHexString());
      const successorName = (await this.userService.findById(successorId))?.name ?? '';
      await this.groupRepo.updateMemberRole(groupId, successorId, Role.GroupAdmin);
      await this.groupRepo.deleteMember(groupId, targetUserId);
      await this.adminRepo.insertAuditEntry(
        auditEntry({
          action: AuditAction.Succession,
          groupId,
          groupName,
          deletedUserId: targetUserId,
          deletedUserName,
          successorUserId: successorId,
          successorUserName: successorName,
          performedBy: callerId,
          performedByName,
        }),
      );
    }

    for (const { groupId, groupName } of plan.autoDelete) {
      await purgeGroupData(this.db, groupId);
      await this.adminRepo.insertAuditEntry(
        auditEntry({
          action: AuditAction.GroupAutoDeleted,
          groupId,
          groupName,
          deletedUserId: targetUserId,
          deletedUserName,
          performedBy: callerId,
          performedByName,
        }),
      );
    }

    for (const groupId of plan.plainRemovals) {
      await this.groupRepo.deleteMember(groupId, targetUserId);
    }

    await this.userService.delete(targetUserId);
  }

  async listUsers(callerId, search) {
    await this.rbac.requireSystemAdmin(callerId);
    return this.userService.listAll(search);
  }

  // Grants the target user the global System Admin role, audit-logged.
  async promoteUser(callerId, targetUserId) {
    await this.rbac.requireSystemAdmin(callerId);
    const targetUser = await this.userService.findById(targetUserId);
    if (!targetUser) throw ApiError.notFound();
    if (targetUser.global_role === GlobalRole.SystemAdmin) {
      throw ApiError.conflict('user is already a System Admin');
    }
    const performedByName = (await this.userService.findById(callerId))?.name ?? '';

    await this.userService.updateGlobalRole(targetUserId, GlobalRole.SystemAdmin);
    await this.adminRepo.insertAuditEntry(
      auditEntry({
        action: AuditAction.Promotion,
        targetUserId,
        targetUserName: targetUser.name,
        performedBy: callerId,
        performedByName,
      }),
    );
  }

  // Revokes the target user's System Admin role. Refuses (409) to demote the
  // last remaining System Admin. Self-demotion is allowed as long as another
  // System Admin still exists afterward.
  async demoteUser(callerId, targetUserId) {
    await this.rbac.requireSystemAdmin(callerId);
    const targetUser = await this.userService.findById(targetUserId);
    if (!targetUser) throw ApiError.notFound();
    if (targetUser.global_role !== GlobalRole.SystemAdmin) {
      throw ApiError.conflict('user is not a System Admin');
    }
    if ((await this.userService.countSystemAdmins()) <= 1) {
      throw ApiError.conflict('cannot demote the last System Admin');
    }
    const performedByName = (await this.userService.findById(callerId))?.name ?? '';

    await this.userService.clearGlobalRole(targetUserId);
    await this.adminRepo.insertAuditEntry(
      auditEntry({
        action: AuditAction.Demotion,
        targetUserId,
        targetUserName: targetUser.name,
        performedBy: callerId,
        performedByName,
      }),
    );
  }

  // Read-only view of the audit trail. Filters are optional and independent;
  // results are newest-first.
  async listAuditLog(callerId, groupId, deletedUserId) {
    await this.rbac.requireSystemAdmin(callerId);
    return (await this.adminRepo.listAuditLog(groupId, deletedUserId)).map(auditEntryResponse);
  }

  async listGroups(callerId, search) {
    await this.rbac.requireSystemAdmin(callerId);
    return (await this.groupRepo.listAllGroups(search)).map(groupResponse);
  }

  // No membership or succession check — deleting the whole group removes the
  // "at least one Group Admin" invariant along with it.
  async deleteGroup(callerId, groupId) {
    await this.rbac.requireSystemAdmin(callerId);
    if (!(await purgeGroupData(this.db, groupId))) throw ApiError.notFound();
  }

  async enrichMembers(members) {
    const result = [];
    for (const member of members) {
      result.push(memberResponse(member, await this.userService.findById(member.user_id)));
    }
    return result;
  }

  // Walks every group the target belongs to and classifies each one:
  //   blocked       — sole Group Admin, other members exist: needs a successor
  //   autoDelete    — sole Group Admin, no other members: group is deleted
  //   plainRemovals — Contributor, or one of several admins: just leaves
  async buildPlan(targetUserId) {
    const groups = await this.groupRepo.listGroupsForUser(targetUserId);
    const plan = { blocked: [], autoDelete: [], plainRemovals: [] };

    for (const group of groups) {
      const groupId = group._id;
      const membership = await this.groupRepo.findMember(groupId, targetUserId);
      if (!membership) throw ApiError.internal();

      if (membership.role !== Role.GroupAdmin) {
        plan.plainRemovals.push(groupId);
        continue;
      }
      if ((await this.groupRepo.countGroupAdmins(groupId)) > 1) {
        plan.plainRemovals.push(groupId);
        continue;
      }

      const others = (await this.groupRepo.listMembers(groupId)).filter(
        (m) => !m.user_id.equals(targetUserId),
      );
      if (others.length === 0) plan.autoDelete.push({ groupId, groupName: group.name });
      else plan.blocked.push({ groupId, groupName: group.name, others });
    }

    return plan;
  }
}
