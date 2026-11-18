import { ActivityEventType, LinkKind } from '../activity/models.js';
import { ActivityRepository } from '../activity/repository.js';
import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { formatDateTime } from '../rust.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { parseObjectId } from '../utils.js';
import { RelationType, describeLabel, labelFor } from './models.js';
import { LinkRepository } from './repository.js';

export class LinkService {
  constructor(db) {
    this.repo = new LinkRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.activityRepo = new ActivityRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  // Any group member may add a link — no owner/admin split.
  async createLink(userId, groupId, ticketId, input) {
    await this.rbac.requireMember(groupId, userId);

    const targetTicketId = parseObjectId(input.target_ticket_id);
    if (!targetTicketId) throw ApiError.validation('invalid target_ticket_id');
    if (targetTicketId.equals(ticketId)) {
      throw ApiError.validation('a ticket cannot be linked to itself');
    }

    const source = await this.requireTicketInGroup(groupId, ticketId);
    // target comes from the body, not the path, so an unknown/cross-group id
    // is a 400 (bad input) rather than a 404.
    const target = await this.ticketRepo.findById(groupId, targetTicketId);
    if (!target) throw ApiError.validation('target ticket not found in this group');

    if (await this.repo.exists(groupId, ticketId, targetTicketId, input.relation_type)) {
      throw ApiError.conflict('this link already exists');
    }
    // relates_to is symmetric — one document represents the pair in either
    // direction, so the inverse must be checked too.
    if (
      input.relation_type === RelationType.RelatesTo &&
      (await this.repo.exists(groupId, targetTicketId, ticketId, RelationType.RelatesTo))
    ) {
      throw ApiError.conflict('this link already exists');
    }

    const link = await this.repo.insert({
      groupId,
      sourceTicketId: ticketId,
      targetTicketId,
      relationType: input.relation_type,
      createdBy: userId,
    });

    await this.recordLinkActivity(groupId, link, source, target, userId, ActivityEventType.LinkAdded);
    return this.enrich(link, ticketId, target);
  }

  // Read-only for every group member.
  async listLinks(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicketInGroup(groupId, ticketId);

    const links = await this.repo.listByTicket(groupId, ticketId);
    const result = [];
    for (const link of links) {
      const otherTicketId = link.source_ticket_id.equals(ticketId)
        ? link.target_ticket_id
        : link.source_ticket_id;
      // Links are cascade-deleted with either side, so the other ticket always
      // exists; a missing one is an internal inconsistency.
      const other = await this.ticketRepo.findById(groupId, otherTicketId);
      if (!other) throw ApiError.internal();
      result.push(await this.enrich(link, ticketId, other));
    }
    return result;
  }

  // Owner or Group Admin only.
  async deleteLink(userId, groupId, ticketId, linkId) {
    const member = await this.rbac.requireMember(groupId, userId);
    const link = await this.repo.findById(groupId, linkId);
    if (!link || !(link.source_ticket_id.equals(ticketId) || link.target_ticket_id.equals(ticketId))) {
      throw ApiError.notFound();
    }
    RbacService.requireOwnerOrGroupAdmin(member, link.created_by);

    const source = await this.ticketRepo.findById(groupId, link.source_ticket_id);
    if (!source) throw ApiError.internal();
    const target = await this.ticketRepo.findById(groupId, link.target_ticket_id);
    if (!target) throw ApiError.internal();

    if (!(await this.repo.delete(groupId, linkId))) throw ApiError.notFound();

    await this.recordLinkActivity(groupId, link, source, target, userId, ActivityEventType.LinkRemoved);
  }

  // Logs one entry on each side of the link — being linked from another
  // ticket is a real change to *that* ticket's history too.
  async recordLinkActivity(groupId, link, source, target, actorId, eventType) {
    const sourceText = `${describeLabel(labelFor(link.relation_type, true))} #${target.ticket_number}`;
    const targetText = `${describeLabel(labelFor(link.relation_type, false))} #${source.ticket_number}`;
    const added = eventType === ActivityEventType.LinkAdded;

    await this.activityRepo.insert({
      groupId,
      ticketId: source._id,
      actorId,
      eventType,
      oldValue: added ? null : sourceText,
      newValue: added ? sourceText : null,
      linkKind: LinkKind.Relation,
    });
    await this.activityRepo.insert({
      groupId,
      ticketId: target._id,
      actorId,
      eventType,
      oldValue: added ? null : targetText,
      newValue: added ? targetText : null,
      linkKind: LinkKind.Relation,
    });
  }

  // TicketLinkResponse needs the other ticket's summary fields and the
  // creator's display name, neither of which the link carries.
  async enrich(link, viewpointTicketId, other) {
    const label = labelFor(link.relation_type, link.source_ticket_id.equals(viewpointTicketId));
    const creator = await this.userService.findById(link.created_by);
    return {
      id: link._id ? link._id.toHexString() : '',
      group_id: link.group_id.toHexString(),
      // Already resolved for the ticket the request was scoped to.
      label,
      other_ticket_id: other._id ? other._id.toHexString() : '',
      other_ticket_number: other.ticket_number,
      other_ticket_title: other.title,
      other_ticket_status: other.status,
      other_ticket_priority: other.priority,
      created_by: link.created_by.toHexString(),
      created_by_name: creator ? creator.name : '',
      created_at: formatDateTime(link.created_at),
    };
  }

  async requireTicketInGroup(groupId, ticketId) {
    const ticket = await this.ticketRepo.findById(groupId, ticketId);
    if (!ticket) throw ApiError.notFound();
    return ticket;
  }
}
