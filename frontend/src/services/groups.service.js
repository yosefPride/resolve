import api from '../lib/axios';

export function listGroups() {
  return api.get('/groups');
}

export function createGroup(name) {
  return api.post('/groups', { name });
}

export function getGroup(groupId) {
  return api.get(`/groups/${groupId}`);
}

export function renameGroup(groupId, name) {
  return api.patch(`/groups/${groupId}`, { name });
}

export function deleteGroup(groupId) {
  return api.delete(`/groups/${groupId}`);
}

export function listMembers(groupId) {
  return api.get(`/groups/${groupId}/users`);
}

export function lookupUserByEmail(groupId, email) {
  return api
    .get(`/groups/${groupId}/users/lookup`, { params: { email } });
}

export function addMember(groupId, userId, role) {
  return api
    .post(`/groups/${groupId}/users`, { user_id: userId, role });
}

export function updateMemberRole(groupId, userId, role) {
  return api
    .patch(`/groups/${groupId}/users/${userId}`, { role });
}

export function removeMember(groupId, userId) {
  return api.delete(`/groups/${groupId}/users/${userId}`);
}
