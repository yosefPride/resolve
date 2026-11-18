import { ActivityRepository } from '../activity/repository.js';
import { AiRepository } from '../ai/repository.js';
import { CommentRepository } from '../comment/repository.js';
import { ApiError } from '../errors.js';
import { LinkRepository } from '../link/repository.js';
import { RbacService } from '../rbac/service.js';
import { ReactionRepository } from '../reaction/repository.js';
import { ReferenceRepository } from '../reference/repository.js';
import { formatDateTime } from '../rust.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { Role, groupResponse, memberResponse } from './models.js';
import { GroupRepository } from './repository.js';

// The one place that knows what "a group's data" consists of. Every path that
// destroys a group goes through here — GroupService.deleteGroup (Group Admin
// deleting their own), AdminService.deleteGroup (System Admin deleting any),
// and AdminService.deleteUser (sole-admin auto-deletion). Each keeps its own
// authorization check; only the cascade is shared.
//
// Deletion order is child-to-parent: the group document goes last, so a
// failure partway through leaves the group still resolvable and the cascade
// re-runnable. Sequential writes, not a transaction.
export async function purgeGroupData(db, groupId) {
  const repo = new GroupRepository(db);
  await repo.deleteMembersByGroup(groupId);
  await new TicketRepository(db).deleteByGroup(groupId);
  await new CommentRepository(db).deleteByGroup(groupId);
  await new AiRepository(db).deleteByGroup(groupId);
  await new ActivityRepository(db).deleteByGroup(groupId);
  await new LinkRepository(db).deleteByGroup(groupId);
  await new ReactionRepository(db).deleteByGroup(groupId);
  await new ReferenceRepository(db).deleteByGroup(groupId);
  return repo.deleteGroup(groupId);
}

export class GroupService {
  constructor(db) {
    this.db = db;
    this.repo = new GroupRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.activityRepo = new ActivityRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  // Two sequential writes, not a transaction. If the second write fails, the
  // group is left with no members; low-probability and cheap to fix by hand.
  async createGroup(userId, name) {
    const group = await this.repo.createGroup({ name, ownerId: userId });
    await this.repo.insertMember(group._id, userId, Role.GroupAdmin);
    return groupResponse(group);
  }

  async listMyGroups(userId) {
    const memberships = await this.repo.listMembershipsForUser(userId);
    const result = [];
    for (const membership of memberships) {
      const group = await this.repo.findGroupById(membership.group_id);
      if (!group) throw ApiError.internal();
      const memberCount = await this.repo.countMembers(membership.group_id);
      const openTicketCount = await this.ticketRepo.countOpenByGroup(membership.group_id);
      const latest = await this.activityRepo.findLatestForGroup(membership.group_id);
      result.push({
        id: group._id ? group._id.toHexString() : '',
        name: group.name,
        role: membership.role,
        member_count: memberCount,
        open_ticket_count: openTicketCount,
        // Null for a brand-new group with no tickets yet — every ticket
        // mutation writes a ticket_activity entry, so this is the most recent
        // occurred_at across the whole group.
        last_activity_at: latest ? formatDateTime(latest.occurred_at) : null,
        created_at: formatDateTime(group.created_at),
      });
    }
    return result;
  }

  async getGroup(userId, groupId) {
    await this.rbac.requireMember(groupId, userId);
    const group = await this.repo.findGroupById(groupId);
    if (!group) throw ApiError.notFound();
    return groupResponse(group);
  }

  async renameGroup(userId, groupId, name) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    await this.repo.renameGroup(groupId, name);
    const group = await this.repo.findGroupById(groupId);
    if (!group) throw ApiError.notFound();
    return groupResponse(group);
  }

  async deleteGroup(userId, groupId) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    await purgeGroupData(this.db, groupId);
  }

  async listMembers(userId, groupId) {
    await this.rbac.requireMember(groupId, userId);
    const members = await this.repo.listMembers(groupId);
    const result = [];
    for (const member of members) result.push(await this.enrichMember(member));
    return result;
  }

  // One user lookup per member rather than a $lookup aggregation: matches the
  // rest of the repository layer, fine at expected group sizes.
  async enrichMember(member) {
    const user = await this.userService.findById(member.user_id);
    return memberResponse(member, user);
  }

  // Group Admin only. There is no user directory or join flow — an exact
  // email match is the only way to resolve the user_id addMember needs.
  async lookupUserByEmail(userId, groupId, email) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    const target = await this.userService.findByEmail(email);
    if (!target) throw ApiError.notFound();
    return {
      id: target._id ? target._id.toHexString() : '',
      name: target.name,
      email: target.email,
    };
  }

  async addMember(userId, groupId, targetUserId, role) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    const member = await this.repo.insertMember(groupId, targetUserId, role);
    return this.enrichMember(member);
  }

  async updateMemberRole(userId, groupId, targetUserId, role) {
    await this.rbac.requireGroupAdmin(groupId, userId);

    if (role === Role.Contributor) {
      // Demoting the group's last Group Admin is blocked, same as removing them.
      await this.guardSoleAdminRemoval(groupId, targetUserId);
    } else if (!(await this.repo.findMember(groupId, targetUserId))) {
      throw ApiError.notFound();
    }

    if (!(await this.repo.updateMemberRole(groupId, targetUserId, role))) {
      throw ApiError.notFound();
    }

    const member = await this.repo.findMember(groupId, targetUserId);
    if (!member) throw ApiError.internal();
    return this.enrichMember(member);
  }

  async removeMember(userId, groupId, targetUserId) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    await this.guardSoleAdminRemoval(groupId, targetUserId);
    if (!(await this.repo.deleteMember(groupId, targetUserId))) throw ApiError.notFound();
  }

  async leaveGroup(userId, groupId) {
    await this.guardSoleAdminRemoval(groupId, userId);
    if (!(await this.repo.deleteMember(groupId, userId))) throw ApiError.notFound();
  }

  // Blocks removing/demoting a group's last Group Admin — a successor must be
  // appointed first (docs/rbac.md, "Group Admin Succession").
  async guardSoleAdminRemoval(groupId, targetUserId) {
    const target = await this.repo.findMember(groupId, targetUserId);
    if (!target) throw ApiError.notFound();
    if (target.role === Role.GroupAdmin && (await this.repo.countGroupAdmins(groupId)) <= 1) {
      throw ApiError.conflict(
        'a successor Group Admin must be appointed before the sole Group Admin can be removed',
      );
    }
  }
}
