import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { activityResponse } from './models.js';
import { ActivityRepository } from './repository.js';

export class ActivityService {
  constructor(db) {
    this.repo = new ActivityRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  // Read-only for every group member (same visibility as the ticket itself).
  async listActivity(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    // Confirms ticket_id actually belongs to group_id before returning
    // anything — groupScoped only proves membership in group_id.
    if (!(await this.ticketRepo.findById(groupId, ticketId))) throw ApiError.notFound();

    const entries = await this.repo.listByTicket(groupId, ticketId);
    const result = [];
    for (const entry of entries) {
      // The actor's current display name, resolved at read time so a later
      // name change is reflected.
      const actor = await this.userService.findById(entry.actor_id);
      result.push(activityResponse(entry, actor));
    }
    return result;
  }
}
