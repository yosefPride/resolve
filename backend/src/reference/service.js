import { ActivityEventType, LinkKind } from '../activity/models.js';
import { ActivityRepository } from '../activity/repository.js';
import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { formatDateTime, trim } from '../rust.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { ReferenceRepository } from './repository.js';

export class ReferenceService {
  constructor(db) {
    this.repo = new ReferenceRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.activityRepo = new ActivityRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  // Any group member may attach a reference. `url` is assumed already
  // validated by the handler.
  async createReference(userId, groupId, ticketId, input) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicketInGroup(groupId, ticketId);

    const label = input.label === null ? '' : trim(input.label);
    const reference = await this.repo.insert({
      groupId,
      ticketId,
      label: label !== '' ? label : deriveLabel(input.url),
      url: input.url,
      createdBy: userId,
    });

    await this.recordReferenceActivity(groupId, ticketId, userId, ActivityEventType.LinkAdded, reference.label);
    return this.enrich(reference);
  }

  // Read-only for every group member.
  async listReferences(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicketInGroup(groupId, ticketId);
    const references = await this.repo.listByTicket(groupId, ticketId);
    const result = [];
    for (const reference of references) result.push(await this.enrich(reference));
    return result;
  }

  // Owner or Group Admin only.
  async deleteReference(userId, groupId, ticketId, referenceId) {
    const member = await this.rbac.requireMember(groupId, userId);
    const reference = await this.repo.findById(groupId, ticketId, referenceId);
    if (!reference) throw ApiError.notFound();
    RbacService.requireOwnerOrGroupAdmin(member, reference.created_by);

    if (!(await this.repo.delete(groupId, ticketId, referenceId))) throw ApiError.notFound();

    await this.recordReferenceActivity(groupId, ticketId, userId, ActivityEventType.LinkRemoved, reference.label);
  }

  async recordReferenceActivity(groupId, ticketId, actorId, eventType, label) {
    const added = eventType === ActivityEventType.LinkAdded;
    await this.activityRepo.insert({
      groupId,
      ticketId,
      actorId,
      eventType,
      oldValue: added ? null : label,
      newValue: added ? label : null,
      linkKind: LinkKind.Reference,
    });
  }

  async enrich(reference) {
    const creator = await this.userService.findById(reference.created_by);
    return {
      id: reference._id ? reference._id.toHexString() : '',
      group_id: reference.group_id.toHexString(),
      ticket_id: reference.ticket_id.toHexString(),
      label: reference.label,
      url: reference.url,
      created_by: reference.created_by.toHexString(),
      created_by_name: creator ? creator.name : '',
      created_at: formatDateTime(reference.created_at),
    };
  }

  async requireTicketInGroup(groupId, ticketId) {
    if (!(await this.ticketRepo.findById(groupId, ticketId))) throw ApiError.notFound();
  }
}

// Fallback label when the caller leaves one blank — just the host, e.g.
// "github.com" from "https://github.com/org/repo/pull/12".
function deriveLabel(url) {
  let withoutScheme = url;
  if (url.startsWith('https://')) withoutScheme = url.slice('https://'.length);
  else if (url.startsWith('http://')) withoutScheme = url.slice('http://'.length);
  const hostEnd = withoutScheme.search(/[/?#]/);
  return hostEnd === -1 ? withoutScheme : withoutScheme.slice(0, hostEnd);
}
