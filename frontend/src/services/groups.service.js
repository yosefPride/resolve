import api from '../lib/axios';

export function listGroups() {
  return api.get('/groups').then((res) => res.data);
}

export function createGroup(name) {
  return api.post('/groups', { name }).then((res) => res.data);
}

export function getGroup(groupId) {
  return api.get(`/groups/${groupId}`).then((res) => res.data);
}

export function renameGroup(groupId, name) {
  return api.patch(`/groups/${groupId}`, { name }).then((res) => res.data);
}

export function deleteGroup(groupId) {
  return api.delete(`/groups/${groupId}`).then((res) => res.data);
}

export function listMembers(groupId) {
  return api.get(`/groups/${groupId}/users`).then((res) => res.data);
}

export function lookupUserByEmail(groupId, email) {
  return api
    .get(`/groups/${groupId}/users/lookup`, { params: { email } })
    .then((res) => res.data);
}

export function addMember(groupId, userId, role) {
  return api
    .post(`/groups/${groupId}/users`, { user_id: userId, role })
    .then((res) => res.data);
}

export function updateMemberRole(groupId, userId, role) {
  return api
    .patch(`/groups/${groupId}/users/${userId}`, { role })
    .then((res) => res.data);
}

export function removeMember(groupId, userId) {
  return api.delete(`/groups/${groupId}/users/${userId}`).then((res) => res.data);
}
