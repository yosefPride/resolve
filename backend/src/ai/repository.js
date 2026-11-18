import { insertWithId } from '../utils.js';
import { ChatRole, decodeChatMessage, decodeConversation, decodeInsight } from './models.js';

export class AiRepository {
  constructor(db) {
    this.insights = db.collection('ai_ticket_insights');
    this.chatMessages = db.collection('ai_chat_messages');
    this.conversations = db.collection('ai_conversations');
  }

  // Filtered on group_id as well as ticket_id: a ticket_id from another group
  // simply matches nothing.
  async findInsight(groupId, ticketId) {
    const doc = await this.insights.findOne({ group_id: groupId, ticket_id: ticketId });
    return doc ? decodeInsight(doc) : null;
  }

  // Upserts just the summary field group, leaving any existing analysis
  // fields on the same document untouched.
  async upsertSummary(groupId, ticketId, summary, sourceUpdatedAt) {
    const now = new Date();
    return this.upsert(groupId, ticketId, now, {
      summary,
      summary_source_updated_at: sourceUpdatedAt,
      updated_at: now,
    });
  }

  // Upserts just the analysis field group, leaving any existing summary untouched.
  async upsertAnalysis(groupId, ticketId, analysis, sourceUpdatedAt) {
    const now = new Date();
    return this.upsert(groupId, ticketId, now, {
      severity_prediction: analysis.severityPrediction,
      suggested_fix: analysis.suggestedFix,
      classification: analysis.classification,
      analysis_source_updated_at: sourceUpdatedAt,
      updated_at: now,
    });
  }

  async upsert(groupId, ticketId, now, fields) {
    const doc = await this.insights.findOneAndUpdate(
      { group_id: groupId, ticket_id: ticketId },
      {
        $set: fields,
        $setOnInsert: { group_id: groupId, ticket_id: ticketId, created_at: now },
      },
      { upsert: true, returnDocument: 'after' },
    );
    return decodeInsight(doc);
  }

  // Cascade target for ticket deletion. Messages and conversations are
  // filtered by group_id+ticket_id directly, so a conversation belonging to a
  // deleted user still gets cleaned up.
  async deleteByTicket(groupId, ticketId) {
    const filter = { group_id: groupId, ticket_id: ticketId };
    const insights = (await this.insights.deleteMany(filter)).deletedCount;
    const messages = (await this.chatMessages.deleteMany(filter)).deletedCount;
    const conversations = (await this.conversations.deleteMany(filter)).deletedCount;
    return insights + messages + conversations;
  }

  // Cascade target for group deletion.
  async deleteByGroup(groupId) {
    const filter = { group_id: groupId };
    const insights = (await this.insights.deleteMany(filter)).deletedCount;
    const messages = (await this.chatMessages.deleteMany(filter)).deletedCount;
    const conversations = (await this.conversations.deleteMany(filter)).deletedCount;
    return insights + messages + conversations;
  }

  async createConversation(groupId, ticketId, userId) {
    const now = new Date();
    const conversation = {
      group_id: groupId,
      ticket_id: ticketId,
      user_id: userId,
      title: null,
      created_at: now,
      updated_at: now,
    };
    const id = await insertWithId(this.conversations, conversation);
    return { _id: id, ...conversation };
  }

  // Filtered on user_id too — ownership is enforced by the query itself.
  // Most-recently-active first.
  async listConversations(groupId, ticketId, userId) {
    const docs = await this.conversations
      .find({ group_id: groupId, ticket_id: ticketId, user_id: userId })
      .sort({ updated_at: -1 })
      .toArray();
    return docs.map(decodeConversation);
  }

  // No ownership filter — the service checks group/ticket/user on the result.
  async findConversation(conversationId) {
    const doc = await this.conversations.findOne({ _id: conversationId });
    return doc ? decodeConversation(doc) : null;
  }

  // Bumps updated_at (drives listConversations' sort) on every message, and
  // fills in title only the first time — the "only if still unset" guard
  // lives in the query filter itself.
  async touchConversation(conversationId, titleIfUnset) {
    await this.conversations.updateOne({ _id: conversationId, title: null }, { $set: { title: titleIfUnset } });
    await this.conversations.updateOne({ _id: conversationId }, { $set: { updated_at: new Date() } });
  }

  // Oldest-first.
  async listConversationMessages(conversationId) {
    const docs = await this.chatMessages
      .find({ conversation_id: conversationId })
      .sort({ created_at: 1 })
      .toArray();
    return docs.map(decodeChatMessage);
  }

  async insertChatMessage(conversationId, groupId, ticketId, role, userId, content) {
    const message = {
      conversation_id: conversationId,
      group_id: groupId,
      ticket_id: ticketId,
      role,
      user_id: userId,
      content,
      created_at: new Date(),
    };
    const id = await insertWithId(this.chatMessages, message);
    return { _id: id, ...message };
  }

  // Deletes one (already ownership-checked) conversation and its messages.
  async deleteConversation(conversationId) {
    const conversation = (await this.conversations.deleteOne({ _id: conversationId })).deletedCount;
    const messages = (await this.chatMessages.deleteMany({ conversation_id: conversationId })).deletedCount;
    return conversation + messages;
  }

  // This user's own chat messages since `since`, across every ticket — the
  // rate limit is per user, so it can't be dodged by spreading messages
  // across tickets.
  countRecentUserMessages(userId, since) {
    return this.chatMessages.countDocuments({
      role: ChatRole.User,
      user_id: userId,
      created_at: { $gte: since },
    });
  }
}
