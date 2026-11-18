import { decodeReaction } from './models.js';

export class ReactionRepository {
  constructor(db) {
    this.reactions = db.collection('comment_reactions');
  }

  // Upsert on the unique (comment_id, user_id) key: a new emoji replaces the
  // user's existing reaction on this comment rather than adding a second row.
  async setReaction({ groupId, ticketId, commentId, userId, emoji }) {
    await this.reactions.replaceOne(
      { comment_id: commentId, user_id: userId },
      {
        group_id: groupId,
        ticket_id: ticketId,
        comment_id: commentId,
        user_id: userId,
        emoji,
        created_at: new Date(),
      },
      { upsert: true },
    );
  }

  async removeReaction(commentId, userId) {
    return (await this.reactions.deleteOne({ comment_id: commentId, user_id: userId })).deletedCount > 0;
  }

  // Every reaction on one comment, for building its summary.
  async listByComment(commentId) {
    return (await this.reactions.find({ comment_id: commentId }).toArray()).map(decodeReaction);
  }

  // Cascade target for a single comment's deletion (both hard-delete and
  // tombstone). Not group/ticket-filtered: only ever called on a comment_id
  // already resolved through CommentRepository.findById.
  async deleteByComment(commentId) {
    return (await this.reactions.deleteMany({ comment_id: commentId })).deletedCount;
  }

  // Cascade target for a single ticket's deletion.
  async deleteByTicket(groupId, ticketId) {
    return (await this.reactions.deleteMany({ group_id: groupId, ticket_id: ticketId })).deletedCount;
  }

  // Cascade target for a whole group's deletion.
  async deleteByGroup(groupId) {
    return (await this.reactions.deleteMany({ group_id: groupId })).deletedCount;
  }
}
