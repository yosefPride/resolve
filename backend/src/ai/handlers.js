import { ApiError } from '../errors.js';
import { charCount, trim } from '../rust.js';
import { json, path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { SendChatMessageRequest } from './models.js';
import { AiService } from './service.js';

// Same limit as a comment — a chat message is free-form user text too.
const MAX_MESSAGE_LEN = 2000;

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

function validateMessage(message) {
  if (trim(message) === '') throw ApiError.validation('message is required');
  if (charCount(message) > MAX_MESSAGE_LEN) {
    throw ApiError.validation(`message must be at most ${MAX_MESSAGE_LEN} characters`);
  }
}

export const summarizeTicket = handler([groupScoped, path], async ({ state }, scoped, [, rawTicketId]) => {
  const ticketId = parseId(rawTicketId);
  const service = AiService.create(state.db, state.config);
  const summary = await service.summarizeTicket(scoped.userId, scoped.groupId, ticketId);
  return HttpResponse.ok().json(summary);
});

export const analyzeTicket = handler([groupScoped, path], async ({ state }, scoped, [, rawTicketId]) => {
  const ticketId = parseId(rawTicketId);
  const service = AiService.create(state.db, state.config);
  const analysis = await service.analyzeTicket(scoped.userId, scoped.groupId, ticketId);
  return HttpResponse.ok().json(analysis);
});

export const createConversation = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId]) => {
    const ticketId = parseId(rawTicketId);
    const service = AiService.create(state.db, state.config);
    const conversation = await service.createConversation(scoped.userId, scoped.groupId, ticketId);
    return HttpResponse.created().json(conversation);
  },
);

export const listConversations = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId]) => {
    const ticketId = parseId(rawTicketId);
    const service = AiService.create(state.db, state.config);
    const conversations = await service.listConversations(scoped.userId, scoped.groupId, ticketId);
    return HttpResponse.ok().json(conversations);
  },
);

export const sendConversationMessage = handler(
  [groupScoped, path, json(SendChatMessageRequest)],
  async ({ state }, scoped, [, rawTicketId, rawConversationId], input) => {
    const ticketId = parseId(rawTicketId);
    const conversationId = parseId(rawConversationId);
    validateMessage(input.message);
    const service = AiService.create(state.db, state.config);
    const result = await service.sendConversationMessage(
      scoped.userId,
      scoped.groupId,
      ticketId,
      conversationId,
      input.message,
    );
    return HttpResponse.created().json(result);
  },
);

export const listConversationMessages = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawConversationId]) => {
    const ticketId = parseId(rawTicketId);
    const conversationId = parseId(rawConversationId);
    const service = AiService.create(state.db, state.config);
    const messages = await service.listConversationMessages(
      scoped.userId,
      scoped.groupId,
      ticketId,
      conversationId,
    );
    return HttpResponse.ok().json(messages);
  },
);

export const deleteConversation = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawConversationId]) => {
    const ticketId = parseId(rawTicketId);
    const conversationId = parseId(rawConversationId);
    const service = AiService.create(state.db, state.config);
    await service.deleteConversation(scoped.userId, scoped.groupId, ticketId, conversationId);
    return HttpResponse.noContent().finish();
  },
);
