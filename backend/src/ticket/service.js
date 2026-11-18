import { ActivityEventType } from '../activity/models.js';
import { ActivityRepository } from '../activity/repository.js';
import { AiRepository } from '../ai/repository.js';
import { CommentRepository } from '../comment/repository.js';
import { ApiError } from '../errors.js';
import { LinkRepository } from '../link/repository.js';
import { RbacService } from '../rbac/service.js';
import { ReactionRepository } from '../reaction/repository.js';
import { ReferenceRepository } from '../reference/repository.js';
import { charCount, formatDateTime, splitWhitespace, trim } from '../rust.js';
import { UserService } from '../user/service.js';
import { levenshteinDistance, parseObjectId } from '../utils.js';
import { TicketRepository } from './repository.js';

const DEFAULT_PER_PAGE = 20n;
const MAX_PER_PAGE = 100n;
const U64_MASK = (1n << 64n) - 1n;

export class TicketService {
  constructor(db) {
    this.repo = new TicketRepository(db);
    this.commentRepo = new CommentRepository(db);
    this.aiRepo = new AiRepository(db);
    this.activityRepo = new ActivityRepository(db);
    this.linkRepo = new LinkRepository(db);
    this.reactionRepo = new ReactionRepository(db);
    this.referenceRepo = new ReferenceRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  async createTicket(userId, groupId, input) {
    await this.rbac.requireMember(groupId, userId);
    const ticketNumber = await this.repo.nextTicketNumber(groupId);
    const ticket = await this.repo.insertTicket({
      groupId,
      ticketNumber,
      title: input.title,
      description: input.description,
      priority: input.priority,
      createdBy: userId,
    });
    await this.activityRepo.insert({
      groupId,
      ticketId: ticket._id,
      actorId: userId,
      eventType: ActivityEventType.TicketCreated,
    });
    return this.enrichTicket(ticket);
  }

  async getTicket(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    const ticket = await this.repo.findById(groupId, ticketId);
    if (!ticket) throw ApiError.notFound();
    return this.enrichTicket(ticket);
  }

  // Returns the serialized TicketListResponse: `page`/`per_page` are u64
  // echoes of the query, so they're kept exact as BigInt.
  async listTickets(userId, groupId, query) {
    await this.rbac.requireMember(groupId, userId);

    let creator = null;
    if (query.creator !== null) {
      creator = parseObjectId(query.creator);
      if (!creator) throw ApiError.validation('invalid creator id');
    }

    let tickets = await this.repo.listByGroup(groupId, query.status, query.priority, creator);

    const term = query.q === null ? '' : trim(query.q);
    if (term !== '') tickets = searchByTitle(tickets, term);

    const total = tickets.length;
    const requestedPage = query.page ?? 1n;
    const page = requestedPage > 1n ? requestedPage : 1n;
    const requestedPerPage = query.per_page ?? DEFAULT_PER_PAGE;
    const perPage =
      requestedPerPage < 1n ? 1n : requestedPerPage > MAX_PER_PAGE ? MAX_PER_PAGE : requestedPerPage;
    // Release-build u64 arithmetic wraps on overflow.
    const start = ((page - 1n) * perPage) & U64_MASK;
    const from = start > BigInt(tickets.length) ? tickets.length : Number(start);

    const items = [];
    for (const ticket of tickets.slice(from, from + Number(perPage))) {
      items.push(await this.enrichTicket(ticket));
    }

    return `{"items":${JSON.stringify(items)},"total":${total},"page":${page},"per_page":${perPage}}`;
  }

  async updateTicket(userId, groupId, ticketId, input) {
    await this.rbac.requireGroupAdmin(groupId, userId);

    // Fetched up front so activity events are built from real before/after
    // values: an event is only recorded for a field whose value actually
    // changes, not merely one the caller included in the request body.
    const current = await this.repo.findById(groupId, ticketId);
    if (!current) throw ApiError.notFound();

    const changes = {};
    // content_updated_at only bumps for title/description/priority — not a
    // status-only change.
    let contentChanged = false;
    const events = [];

    if (input.title !== null) {
      if (input.title !== current.title) {
        events.push([ActivityEventType.TitleChanged, current.title, input.title]);
      }
      changes.title = input.title;
      contentChanged = true;
    }
    if (input.description !== null) {
      // No before/after text stored for description — just the fact.
      if (input.description !== current.description) {
        events.push([ActivityEventType.DescriptionChanged, null, null]);
      }
      changes.description = input.description;
      contentChanged = true;
    }
    if (input.priority !== null) {
      if (input.priority !== current.priority) {
        events.push([ActivityEventType.PriorityChanged, current.priority, input.priority]);
      }
      changes.priority = input.priority;
      contentChanged = true;
    }
    if (input.status !== null) {
      if (input.status !== current.status) {
        events.push([ActivityEventType.StatusChanged, current.status, input.status]);
      }
      changes.status = input.status;
    }
    const now = new Date();
    changes.updated_at = now;
    if (contentChanged) changes.content_updated_at = now;

    const ticket = await this.repo.updateTicket(groupId, ticketId, changes);
    if (!ticket) throw ApiError.notFound();

    for (const [eventType, oldValue, newValue] of events) {
      await this.activityRepo.insert({
        groupId,
        ticketId,
        actorId: userId,
        eventType,
        oldValue,
        newValue,
      });
    }

    return this.enrichTicket(ticket);
  }

  async deleteTicket(userId, groupId, ticketId) {
    await this.rbac.requireGroupAdmin(groupId, userId);
    // Existence check first (so a bogus ticket_id 404s before any write), then
    // the cascades, then the ticket document last — so a mid-failure leaves
    // the ticket still resolvable and this call re-runnable.
    if (!(await this.repo.findById(groupId, ticketId))) throw ApiError.notFound();
    await this.commentRepo.deleteByTicket(groupId, ticketId);
    // Every comment is gone via the cascade above, so their reactions would
    // otherwise be orphaned.
    await this.reactionRepo.deleteByTicket(groupId, ticketId);
    await this.aiRepo.deleteByTicket(groupId, ticketId);
    await this.activityRepo.deleteByTicket(groupId, ticketId);
    // Removes links on either side, so a deleted ticket never leaves a
    // dangling relation on the other end.
    await this.linkRepo.deleteByTicket(groupId, ticketId);
    await this.referenceRepo.deleteByTicket(groupId, ticketId);
    await this.repo.deleteTicket(groupId, ticketId);
  }

  // TicketResponse needs the creator's display name, which the ticket doesn't
  // carry; a deleted account reads as an empty name.
  async enrichTicket(ticket) {
    const creator = await this.userService.findById(ticket.created_by);
    return {
      id: ticket._id ? ticket._id.toHexString() : '',
      group_id: ticket.group_id.toHexString(),
      ticket_number: ticket.ticket_number,
      title: ticket.title,
      description: ticket.description,
      status: ticket.status,
      priority: ticket.priority,
      created_by: ticket.created_by.toHexString(),
      created_by_name: creator ? creator.name : '',
      created_at: formatDateTime(ticket.created_at),
      updated_at: formatDateTime(ticket.updated_at),
    };
  }
}

// Case-insensitive substring match on title first; if that finds nothing,
// falls back to typo-tolerant matching (edit distance against each word in
// the title, so a typo in one word of a multi-word title still hits).
function searchByTitle(tickets, term) {
  const needle = term.toLowerCase();
  const substringMatches = tickets.filter((t) => t.title.toLowerCase().includes(needle));
  if (substringMatches.length > 0) return substringMatches;

  // Allow roughly one edit per three characters of the query (min 1), so
  // short queries still require a near-exact match.
  const maxDistance = Math.max(Math.floor(charCount(needle) / 3), 1);
  const scored = [];
  for (const ticket of tickets) {
    const distances = splitWhitespace(ticket.title.toLowerCase()).map((word) =>
      levenshteinDistance(word, needle),
    );
    const best = distances.length > 0 ? Math.min(...distances) : Infinity;
    if (best <= maxDistance) scored.push([best, ticket]);
  }
  scored.sort((a, b) => a[0] - b[0]); // stable, like sort_by_key
  return scored.map(([, ticket]) => ticket);
}
