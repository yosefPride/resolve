import { ApiError } from '../errors.js';
import { Role } from '../group/models.js';
import { GroupRepository } from '../group/repository.js';
import { GlobalRole } from '../user/models.js';
import { UserService } from '../user/service.js';

// Central home for the reusable RBAC primitives. These answer a single
// question — "what is this user's relationship to this group (or the
// system)?" — and carry no feature-specific logic.
//
// What deliberately does NOT live here: group isolation (enforced by the
// repository-level group_id filter on every query) and the
// sole-Group-Admin succession guard (GroupService).
export class RbacService {
  constructor(db) {
    this.groupRepo = new GroupRepository(db);
    this.userService = new UserService(db);
  }

  // Not a member -> Forbidden rather than NotFound, deliberately: this avoids
  // telling a non-member whether the group id even exists. Returns the
  // membership (with its role) so callers can branch on the role.
  async requireMember(groupId, userId) {
    const member = await this.groupRepo.findMember(groupId, userId);
    if (!member) throw ApiError.forbidden();
    return member;
  }

  async requireGroupAdmin(groupId, userId) {
    const member = await this.requireMember(groupId, userId);
    if (member.role !== Role.GroupAdmin) throw ApiError.forbidden();
    return member;
  }

  // Global-scope check, independent of any group. A missing user maps to
  // Forbidden (same as a non-admin) so a stale/forged token can't probe user
  // existence.
  async requireSystemAdmin(userId) {
    const caller = await this.userService.findById(userId);
    if (!caller || caller.global_role !== GlobalRole.SystemAdmin) throw ApiError.forbidden();
  }

  // A Contributor may act only on resources they created, a Group Admin on
  // any resource in the group. Pure, no DB access.
  static requireOwnerOrGroupAdmin(member, resourceOwnerId) {
    if (member.role === Role.GroupAdmin || member.user_id.equals(resourceOwnerId)) return;
    throw ApiError.forbidden();
  }
}
