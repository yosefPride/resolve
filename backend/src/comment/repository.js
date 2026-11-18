import { insertWithId } from '../utils.js';
import { DELETED_CONTENT_PLACEHOLDER, decodeComment } from './models.js';

export class CommentRepository {
  constructor(db) {
    this.comments = db.collection('comments');
  }

  async insertComment({ groupId, ticketId, parentCommentId, userId, content }) {
    const comment = {
      group_id: groupId,
      ticket_id: ticketId,
      parent_comment_id: parentCommentId,
      user_id: userId,
      content,
      is_deleted: false,
      created_at: new Date(),
    };
    const id = await insertWithId(this.comments, comment);
    return { _id: id, ...comment };
  }

  // Filtered on group_id and ticket_id as well as _id: a comment_id from
  // another group/ticket simply matches nothing.
  async findById(groupId, ticketId, commentId) {
    const doc = await this.comments.findOne({ _id: commentId, group_id: groupId, ticket_id: ticketId });
    return doc ? decodeComment(doc) : null;
  }

  // Every comment for a ticket, oldest first — a discussion thread is read in
  // full. Includes tombstoned comments so their surviving replies still have
  // a parent to render against.
  async listByTicket(groupId, ticketId) {
    const docs = await this.comments
      .find({ group_id: groupId, ticket_id: ticketId })
      .sort({ created_at: 1 })
      .toArray();
    return docs.map(decodeComment);
  }

  // Whether any other comment replies to this one — decides hard vs. soft
  // delete in CommentService.deleteComment.
  async hasReplies(commentId) {
    return (await this.comments.countDocuments({ parent_comment_id: commentId })) > 0;
  }

  async hardDelete(groupId, ticketId, commentId) {
    const result = await this.comments.deleteOne({ _id: commentId, group_id: groupId, ticket_id: ticketId });
    return result.deletedCount > 0;
  }

  // Tombstone: keeps the document (so replies' parent_comment_id stays
  // valid) but clears the visible content and flips is_deleted.
  async softDelete(groupId, ticketId, commentId) {
    const result = await this.comments.updateOne(
      { _id: commentId, group_id: groupId, ticket_id: ticketId },
      { $set: { is_deleted: true, content: DELETED_CONTENT_PLACEHOLDER } },
    );
    return result.modifiedCount > 0;
  }

  // Cascade target for a single ticket's deletion: unconditional hard delete
  // of every comment on the ticket, tombstones included.
  async deleteByTicket(groupId, ticketId) {
    return (await this.comments.deleteMany({ group_id: groupId, ticket_id: ticketId })).deletedCount;
  }

  // Cascade target for a whole group's deletion.
  async deleteByGroup(groupId) {
    return (await this.comments.deleteMany({ group_id: groupId })).deletedCount;
  }
}
