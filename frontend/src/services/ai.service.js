import api from '../lib/axios';

// Doubles as both trigger and read: a cache hit returns the existing
// insight (cached: true) without calling Gemini again, a miss generates
// and stores one. No separate GET endpoint exists.
export function summarizeTicket(groupId, ticketId) {
  return api.post(`/ai/groups/${groupId}/tickets/${ticketId}/summarize`).then((res) => res.data);
}

// Cached independently from summarize — calling one does not affect the other.
export function analyzeTicket(groupId, ticketId) {
  return api.post(`/ai/groups/${groupId}/tickets/${ticketId}/analyze`).then((res) => res.data);
}

// Group Admin only (backend returns 403 for contributors). Time-based cache:
// fresh for 1 hour, so a same-hour call is cheap and returns cached: true.
export function generateGroupReport(groupId) {
  return api.post(`/ai/groups/${groupId}/report`).then((res) => res.data);
}

// Oldest-first, full thread — like listComments, no pagination.
export function listChatMessages(groupId, ticketId) {
  return api.get(`/ai/groups/${groupId}/tickets/${ticketId}/chat`).then((res) => res.data);
}

// Returns { user_message, assistant_message } (both persisted, with real ids
// and timestamps) rather than just the assistant reply, so the caller never
// has to fabricate the user's own message to render it immediately.
export function sendChatMessage(groupId, ticketId, message) {
  return api
    .post(`/ai/groups/${groupId}/tickets/${ticketId}/chat`, { message })
    .then((res) => res.data);
}

export function clearChat(groupId, ticketId) {
  return api.delete(`/ai/groups/${groupId}/tickets/${ticketId}/chat`).then((res) => res.data);
}
