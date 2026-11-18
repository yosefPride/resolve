import { ActivityEventType } from '../activity/models.js';
import { ActivityRepository } from '../activity/repository.js';
import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { ReactionRepository } from '../reaction/repository.js';
import { summarizeReactions } from '../reaction/service.js';
import { formatDateTime } from '../rust.js';
import { TicketStatus } from '../ticket/models.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { CommentRepository } from './repository.js';

export class CommentService {
  constructor(db) {
    this.repo = new CommentRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.activityRepo = new ActivityRepository(db);
    this.reactionRepo = new ReactionRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
  }

  // Any group member may comment on a ticket — unlike tickets, there's no
  // owner/admin split on who may create one.
  async createComment(userId, groupId, ticketId, content, parentCommentId) {
    await this.rbac.requireMember(groupId, userId);
    // Without this, a member of group_id could pass another group's ticket_id
    // and have a comment recorded against a ticket that isn't in their group
    // — groupScoped only proves membership in group_id.
    const ticket = await this.requireTicketInGroup(groupId, ticketId);

    // A closed ticket is read-only for discussion: the thread stays visible
    // and existing comments can still be deleted, but no new comment may be
    // added. Conflict rather than Forbidden — the ticket's state rejects this.
    if (ticket.status === TicketStatus.Closed) {
      throw ApiError.conflict('cannot comment on a closed ticket');
    }

    // The parent must be a comment on this same ticket — otherwise a reply
    // could dangle against an id from a different ticket or group.
    if (parentCommentId !== null && !(await this.repo.findById(groupId, ticketId, parentCommentId))) {
      throw ApiError.validation('parent comment not found on this ticket');
    }

    const comment = await this.repo.insertComment({
      groupId,
      ticketId,
      parentCommentId,
      userId,
      content,
    });
    await this.activityRepo.insert({
      groupId,
      ticketId,
      actorId: userId,
      eventType: ActivityEventType.CommentAdded,
      commentId: comment._id,
    });
    return this.enrichComment(comment, userId);
  }

  // Full thread in one response, oldest-first; the frontend builds the reply
  // tree from each item's parent_comment_id.
  async listComments(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicketInGroup(groupId, ticketId);
    const comments = await this.repo.listByTicket(groupId, ticketId);
    const result = [];
    for (const comment of comments) result.push(await this.enrichComment(comment, userId));
    return result;
  }

  // Owner or Group Admin only. A comment with replies is tombstoned so those
  // replies keep a valid parent; a leaf comment is hard-deleted.
  async deleteComment(userId, groupId, ticketId, commentId) {
    const member = await this.rbac.requireMember(groupId, userId);
    const comment = await this.repo.findById(groupId, ticketId, commentId);
    if (!comment) throw ApiError.notFound();
    RbacService.requireOwnerOrGroupAdmin(member, comment.user_id);

    const changed = (await this.repo.hasReplies(commentId))
      ? await this.repo.softDelete(groupId, ticketId, commentId)
      : await this.repo.hardDelete(groupId, ticketId, commentId);
    if (!changed) throw ApiError.notFound();
    // Cleared on both branches: nothing is left for a reaction to mean
    // anything against.
    await this.reactionRepo.deleteByComment(commentId);
    // Recorded for both the hard-delete and tombstone paths.
    await this.activityRepo.insert({
      groupId,
      ticketId,
      actorId: userId,
      eventType: ActivityEventType.CommentDeleted,
      commentId,
    });
  }

  // Confirms ticket_id belongs to group_id before any comment read or write
  // against it; returns the ticket so createComment can check its status.
  async requireTicketInGroup(groupId, ticketId) {
    const ticket = await this.ticketRepo.findById(groupId, ticketId);
    if (!ticket) throw ApiError.notFound();
    return ticket;
  }

  async enrichComment(comment, viewerId) {
    const author = await this.userService.findById(comment.user_id);
    if (!comment._id) throw ApiError.internal();
    const reactionRows = await this.reactionRepo.listByComment(comment._id);
    return {
      id: comment._id.toHexString(),
      group_id: comment.group_id.toHexString(),
      ticket_id: comment.ticket_id.toHexString(),
      parent_comment_id: comment.parent_comment_id ? comment.parent_comment_id.toHexString() : null,
      user_id: comment.user_id.toHexString(),
      user_name: author ? author.name : '',
      content: comment.content,
      is_deleted: comment.is_deleted,
      created_at: formatDateTime(comment.created_at),
      // Per-emoji counts plus whether the viewer is behind one of them.
      reactions: summarizeReactions(reactionRows, viewerId),
    };
  }
}
