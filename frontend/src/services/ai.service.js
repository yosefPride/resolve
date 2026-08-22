import api from '../lib/axios';

// Doubles as both trigger and read: a cache hit returns the existing
// insight (cached: true) without calling Gemini again, a miss generates
// and stores one. No separate GET endpoint exists.
export function summarizeTicket(groupId, ticketId) {
  return api.post(`/ai/groups/${groupId}/tickets/${ticketId}/summarize`);
}

// Cached independently from summarize — calling one does not affect the other.
export function analyzeTicket(groupId, ticketId) {
  return api.post(`/ai/groups/${groupId}/tickets/${ticketId}/analyze`);
}

// AI chat is now private, per-user conversations (a ticket can have many),
// not a single group-shared thread — these replace the old listChatMessages/
// sendChatMessage/clearChat trio.

export function createConversation(groupId, ticketId) {
  return api
    .post(`/ai/groups/${groupId}/tickets/${ticketId}/conversations`);
}

// Most-recently-active first — the backend already sorts by updated_at desc.
export function listConversations(groupId, ticketId) {
  return api.get(`/ai/groups/${groupId}/tickets/${ticketId}/conversations`);
}

// Oldest-first, full conversation — like listComments, no pagination.
export function listConversationMessages(groupId, ticketId, conversationId) {
  return api
    .get(`/ai/groups/${groupId}/tickets/${ticketId}/conversations/${conversationId}/messages`);
}

// Returns { user_message, assistant_message } (both persisted, with real ids
// and timestamps) rather than just the assistant reply, so the caller never
// has to fabricate the user's own message to render it immediately.
export function sendConversationMessage(groupId, ticketId, conversationId, message) {
  return api
    .post(`/ai/groups/${groupId}/tickets/${ticketId}/conversations/${conversationId}/messages`, {
      message,
    });
}

export function deleteConversation(groupId, ticketId, conversationId) {
  return api
    .delete(`/ai/groups/${groupId}/tickets/${ticketId}/conversations/${conversationId}`);
}
