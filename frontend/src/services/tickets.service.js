import api from '../lib/axios';

// filters: { q?, status?, priority?, creator?, page?, perPage? } — all
// optional/independent. Omitted keys are left off the query string so the
// backend applies its own defaults (page 1, per_page 20).
export function listTickets(groupId, filters = {}) {
  const params = {};
  const term = filters.q?.trim();
  if (term) params.q = term;
  if (filters.status) params.status = filters.status;
  if (filters.priority) params.priority = filters.priority;
  if (filters.creator) params.creator = filters.creator;
  if (filters.page) params.page = filters.page;
  if (filters.perPage) params.per_page = filters.perPage;
  return api.get(`/groups/${groupId}/tickets`, { params }).then((res) => res.data);
}

export function createTicket(groupId, { title, description, priority }) {
  return api
    .post(`/groups/${groupId}/tickets`, { title, description, priority })
    .then((res) => res.data);
}

export function getTicket(groupId, ticketId) {
  return api.get(`/groups/${groupId}/tickets/${ticketId}`).then((res) => res.data);
}

// changes: { title?, description?, priority?, status? } — at least one
// required (the backend rejects an empty body).
export function updateTicket(groupId, ticketId, changes) {
  return api.patch(`/groups/${groupId}/tickets/${ticketId}`, changes).then((res) => res.data);
}

export function deleteTicket(groupId, ticketId) {
  return api.delete(`/groups/${groupId}/tickets/${ticketId}`).then((res) => res.data);
}
