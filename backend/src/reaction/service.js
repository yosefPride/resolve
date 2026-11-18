import { CommentRepository } from '../comment/repository.js';
import { ApiError } from '../errors.js';
import { RbacService } from '../rbac/service.js';
import { compareStr } from '../rust.js';
import { ReactionRepository } from './repository.js';

export class ReactionService {
  constructor(db) {
    this.repo = new ReactionRepository(db);
    this.commentRepo = new CommentRepository(db);
    this.rbac = new RbacService(db);
  }

  // Any group member may react. Deliberately no closed-ticket lock: a
  // reaction isn't new discussion, so a closed ticket's thread stays reactable.
  async setReaction(userId, groupId, ticketId, commentId, emoji) {
    await this.rbac.requireMember(groupId, userId);
    // Filtered on {_id, group_id, ticket_id}, so this alone proves comment_id
    // genuinely belongs to ticket_id within group_id.
    if (!(await this.commentRepo.findById(groupId, ticketId, commentId))) throw ApiError.notFound();
    await this.repo.setReaction({ groupId, ticketId, commentId, userId, emoji });
    return this.summarize(commentId, userId);
  }

  async removeReaction(userId, groupId, ticketId, commentId) {
    await this.rbac.requireMember(groupId, userId);
    if (!(await this.commentRepo.findById(groupId, ticketId, commentId))) throw ApiError.notFound();
    await this.repo.removeReaction(commentId, userId);
    return this.summarize(commentId, userId);
  }

  async summarize(commentId, viewerId) {
    return summarizeReactions(await this.repo.listByComment(commentId), viewerId);
  }
}

// Shared with CommentService.enrichComment. Emoji are ordered by their UTF-8
// bytes (the Rust BTreeMap order), so the reaction bar doesn't reshuffle on
// every refetch.
export function summarizeReactions(rows, viewerId) {
  const counts = new Map();
  for (const row of rows) {
    const entry = counts.get(row.emoji) ?? { count: 0, reactedByMe: false };
    entry.count += 1;
    if (row.user_id.equals(viewerId)) entry.reactedByMe = true;
    counts.set(row.emoji, entry);
  }
  return [...counts.entries()]
    .sort(([a], [b]) => compareStr(a, b))
    .map(([emoji, { count, reactedByMe }]) => ({ emoji, count, reacted_by_me: reactedByMe }));
}
