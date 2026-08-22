import api from '../lib/axios';

export function listReferences(groupId, ticketId) {
  return api.get(`/groups/${groupId}/tickets/${ticketId}/references`);
}

// A blank label is sent as null (rather than omitted) so it's unambiguous —
// the backend falls back to the URL's host when label is absent/null.
export function createReference(groupId, ticketId, { label, url }) {
  return api
    .post(`/groups/${groupId}/tickets/${ticketId}/references`, {
      label: label?.trim() || null,
      url,
    });
}

export function deleteReference(groupId, ticketId, referenceId) {
  return api
    .delete(`/groups/${groupId}/tickets/${ticketId}/references/${referenceId}`);
}
