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
