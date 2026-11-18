import { datetime, model, objectId, oneOf, option, string } from '../db/model.js';
import { formatDateTime } from '../rust.js';
import * as t from '../serde/types.js';

// One document per ticket, upserted in place by summarize/analyze. Each field
// group snapshots the ticket's content_updated_at when it was generated
// (summary_source_updated_at / analysis_source_updated_at): a group is fresh
// iff its values are present AND that snapshot still matches the ticket — so
// an edit invalidates the cache, but closing/reopening a ticket doesn't.
export const decodeInsight = model('AiTicketInsight', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  summary: option(string),
  summary_source_updated_at: option(datetime),
  severity_prediction: option(string),
  suggested_fix: option(string),
  classification: option(string),
  analysis_source_updated_at: option(datetime),
  created_at: datetime,
  updated_at: datetime,
});

const sameTime = (a, b) => a !== null && a.getTime() === b.getTime();

export function isSummaryFresh(insight, ticketContentUpdatedAt) {
  return insight.summary !== null && sameTime(insight.summary_source_updated_at, ticketContentUpdatedAt);
}

export function isAnalysisFresh(insight, ticketContentUpdatedAt) {
  return (
    insight.severity_prediction !== null &&
    insight.suggested_fix !== null &&
    insight.classification !== null &&
    sameTime(insight.analysis_source_updated_at, ticketContentUpdatedAt)
  );
}

// "assistant", not "ai" or "gemini": keeps the field provider-agnostic.
export const ChatRole = { User: 'user', Assistant: 'assistant' };

// A private, per-user conversation on a ticket — the owner (user_id) is the
// only caller who may ever read or delete it; there's no group-admin
// fallback. title starts null and is filled in once, from the first message.
export const decodeConversation = model('AiConversation', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  user_id: objectId,
  title: option(string),
  created_at: datetime,
  updated_at: datetime,
});

// One document per message. user_id is null for an assistant message.
// group_id/ticket_id stay on the message so cascade deletes can filter on
// them directly.
export const decodeChatMessage = model('ChatMessage', {
  _id: option(objectId),
  conversation_id: objectId,
  group_id: objectId,
  ticket_id: objectId,
  role: oneOf(Object.values(ChatRole)),
  user_id: option(objectId),
  content: string,
  created_at: datetime,
});

// No group_id/ticket_id/user_id: the caller already knows them from the URL
// and is always the owner.
export function conversationResponse(conversation) {
  return {
    id: conversation._id ? conversation._id.toHexString() : '',
    title: conversation.title,
    created_at: formatDateTime(conversation.created_at),
    updated_at: formatDateTime(conversation.updated_at),
  };
}

export const SendChatMessageRequest = t.struct('SendChatMessageRequest', { message: t.string });
