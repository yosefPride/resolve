import api from '../lib/axios';

// Newest-first, read-only — every entry is a side effect of a ticket/comment/
// link/reference mutation elsewhere in the API, never written directly.
export function listActivity(groupId, ticketId) {
  return api.get(`/groups/${groupId}/tickets/${ticketId}/activity`);
}
