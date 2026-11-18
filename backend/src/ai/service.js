import { ApiError } from '../errors.js';
import { log } from '../logging.js';
import { RbacService } from '../rbac/service.js';
import { charCount, formatDateTime, takeChars, trim } from '../rust.js';
import { TicketRepository } from '../ticket/repository.js';
import { UserService } from '../user/service.js';
import { GeminiClient } from './client.js';
import { ChatRole, conversationResponse, isAnalysisFresh, isSummaryFresh } from './models.js';
import { AiRepository } from './repository.js';

// 10 messages per hour, scoped per user rather than per ticket — a guardrail
// against one person hammering the endpoint.
const CHAT_RATE_LIMIT = 10;
const CHAT_RATE_WINDOW_MILLIS = 60 * 60 * 1000;
// How many prior messages are fed back to Gemini as context on each turn.
const CHAT_HISTORY_LIMIT = 20;
// A conversation's title comes from its first message (no extra Gemini call);
// this caps how much of it is copied.
const TITLE_MAX_CHARS = 60;

export class AiService {
  // Fails per request rather than at startup — a missing key takes down the
  // AI endpoints, not the whole backend.
  static create(db, config) {
    if (config.geminiApiKey === null) {
      log.error('resolve::ai::service', 'GEMINI_API_KEY is not set — AI endpoints will fail');
      throw ApiError.internal();
    }
    return new AiService(db, new GeminiClient(config.geminiApiKey));
  }

  constructor(db, provider) {
    this.repo = new AiRepository(db);
    this.ticketRepo = new TicketRepository(db);
    this.userService = new UserService(db);
    this.rbac = new RbacService(db);
    this.provider = provider;
  }

  async requireTicket(groupId, ticketId) {
    const ticket = await this.ticketRepo.findById(groupId, ticketId);
    if (!ticket) throw ApiError.notFound();
    return ticket;
  }

  async summarizeTicket(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    const ticket = await this.requireTicket(groupId, ticketId);

    const cached = await this.repo.findInsight(groupId, ticketId);
    if (cached && isSummaryFresh(cached, ticket.content_updated_at)) {
      return { summary: cached.summary, cached: true };
    }

    const summary = await this.provider.summarize(ticket.title, ticket.description);
    await this.repo.upsertSummary(groupId, ticketId, summary, ticket.content_updated_at);
    return { summary, cached: false };
  }

  async analyzeTicket(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    const ticket = await this.requireTicket(groupId, ticketId);

    const cached = await this.repo.findInsight(groupId, ticketId);
    if (cached && isAnalysisFresh(cached, ticket.content_updated_at)) {
      return {
        severity_prediction: cached.severity_prediction,
        suggested_fix: cached.suggested_fix,
        classification: cached.classification,
        cached: true,
      };
    }

    const analysis = await this.provider.analyze(ticket.title, ticket.description);
    await this.repo.upsertAnalysis(groupId, ticketId, analysis, ticket.content_updated_at);
    return {
      severity_prediction: analysis.severityPrediction,
      suggested_fix: analysis.suggestedFix,
      classification: analysis.classification,
      cached: false,
    };
  }

  // Any group member may start a conversation on a ticket — but a
  // conversation is owned by whoever created it: no other member, not even a
  // Group Admin, can list, read, or delete it.
  async createConversation(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicket(groupId, ticketId);
    return conversationResponse(await this.repo.createConversation(groupId, ticketId, userId));
  }

  // Only the caller's own conversations, most-recently-active first.
  async listConversations(userId, groupId, ticketId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireTicket(groupId, ticketId);
    return (await this.repo.listConversations(groupId, ticketId, userId)).map(conversationResponse);
  }

  // A conversation from another ticket/group is NotFound (as if it didn't
  // exist); someone else's conversation is Forbidden.
  async requireOwnConversation(userId, groupId, ticketId, conversationId) {
    const conversation = await this.repo.findConversation(conversationId);
    if (!conversation) throw ApiError.notFound();
    if (!conversation.group_id.equals(groupId) || !conversation.ticket_id.equals(ticketId)) {
      throw ApiError.notFound();
    }
    if (!conversation.user_id.equals(userId)) throw ApiError.forbidden();
    return conversation;
  }

  // Never cacheable: each message is a genuinely new request. Message format
  // validation lives in the handler only.
  async sendConversationMessage(userId, groupId, ticketId, conversationId, message) {
    await this.rbac.requireMember(groupId, userId);
    const ticket = await this.requireTicket(groupId, ticketId);
    await this.requireOwnConversation(userId, groupId, ticketId, conversationId);

    const since = new Date(Date.now() - CHAT_RATE_WINDOW_MILLIS);
    if ((await this.repo.countRecentUserMessages(userId, since)) >= CHAT_RATE_LIMIT) {
      throw ApiError.rateLimited(
        `chat message limit reached (${CHAT_RATE_LIMIT} per hour) — try again later`,
      );
    }

    const history = await this.repo.listConversationMessages(conversationId);
    const reply = await this.provider.chat(
      ticket.title,
      ticket.description,
      buildTranscript(history),
      message,
    );

    const userMessage = await this.repo.insertChatMessage(
      conversationId,
      groupId,
      ticketId,
      ChatRole.User,
      userId,
      message,
    );
    const assistantMessage = await this.repo.insertChatMessage(
      conversationId,
      groupId,
      ticketId,
      ChatRole.Assistant,
      null,
      reply,
    );
    await this.repo.touchConversation(conversationId, deriveTitle(message));

    return {
      user_message: await this.enrichChatMessage(userMessage),
      assistant_message: await this.enrichChatMessage(assistantMessage),
    };
  }

  // Whole conversation, oldest-first, no pagination.
  async listConversationMessages(userId, groupId, ticketId, conversationId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireOwnConversation(userId, groupId, ticketId, conversationId);
    const messages = await this.repo.listConversationMessages(conversationId);
    const result = [];
    for (const message of messages) result.push(await this.enrichChatMessage(message));
    return result;
  }

  // Strictly owner-only.
  async deleteConversation(userId, groupId, ticketId, conversationId) {
    await this.rbac.requireMember(groupId, userId);
    await this.requireOwnConversation(userId, groupId, ticketId, conversationId);
    await this.repo.deleteConversation(conversationId);
  }

  // user_name is null for an assistant message (no user to look up).
  async enrichChatMessage(message) {
    let userName = null;
    if (message.user_id !== null) {
      userName = (await this.userService.findById(message.user_id))?.name ?? null;
    }
    return {
      id: message._id ? message._id.toHexString() : '',
      role: message.role,
      user_id: message.user_id ? message.user_id.toHexString() : null,
      user_name: userName,
      content: message.content,
      created_at: formatDateTime(message.created_at),
    };
  }
}

// Last CHAT_HISTORY_LIMIT messages, oldest-first, as plain text for the prompt.
function buildTranscript(history) {
  return history
    .slice(Math.max(history.length - CHAT_HISTORY_LIMIT, 0))
    .map((message) => `${message.role === ChatRole.User ? 'User' : 'Assistant'}: ${message.content}`)
    .join('\n');
}

// Truncates on a character boundary, never mid-codepoint.
function deriveTitle(content) {
  const trimmed = trim(content);
  if (charCount(trimmed) <= TITLE_MAX_CHARS) return trimmed;
  return `${takeChars(trimmed, TITLE_MAX_CHARS)}…`;
}
