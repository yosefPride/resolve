import api from '../lib/axios';

// A link touches two tickets, but this only ever returns the ones visible
// from `ticketId`'s side — each entry's `label` is already resolved for that
// viewpoint (blocks vs is_blocked_by, etc.), no client-side direction math.
export function listLinks(groupId, ticketId) {
  return api.get(`/groups/${groupId}/tickets/${ticketId}/links`).then((res) => res.data);
}

// relationType: 'blocks' | 'relates_to' | 'duplicates'. `ticketId` is always
// the source of the relation — the backend rejects targetTicketId === ticketId
// (no self-links) and a target from another group.
export function createLink(groupId, ticketId, { targetTicketId, relationType }) {
  return api
    .post(`/groups/${groupId}/tickets/${ticketId}/links`, {
      target_ticket_id: targetTicketId,
      relation_type: relationType,
    })
    .then((res) => res.data);
}

export function deleteLink(groupId, ticketId, linkId) {
  return api
    .delete(`/groups/${groupId}/tickets/${ticketId}/links/${linkId}`)
    .then((res) => res.data);
}
