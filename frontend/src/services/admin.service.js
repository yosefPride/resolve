import api from '../lib/axios';

// System-Admin-only endpoints (backend guards them with SystemAdminUser).
// Non-admin user operations live in users.service.js / groups.service.js.

export function listUsers() {
  return api.get('/admin/users').then((res) => res.data);
}

export function listGroups() {
  return api.get('/admin/groups').then((res) => res.data);
}

export function deleteGroup(groupId) {
  return api.delete(`/admin/groups/${groupId}`).then((res) => res.data);
}

export function deletionCheck(userId) {
  return api.get(`/admin/users/${userId}/deletion-check`).then((res) => res.data);
}

// successors: { [group_id]: successor_user_id } — required for every group in
// the deletion-check's blocked_groups. See docs/api.md POST /admin/users/:id/delete.
export function deleteUser(userId, successors) {
  return api
    .post(`/admin/users/${userId}/delete`, { successors })
    .then((res) => res.data);
}

// filters: { groupId?, userId? } — independent, either/both/neither. userId
// filters by the deleted user. Omitted keys are left off the query string.
export function listAuditLog(filters = {}) {
  const params = {};
  if (filters.groupId) params.group_id = filters.groupId;
  if (filters.userId) params.user_id = filters.userId;
  return api.get('/admin/audit-log', { params }).then((res) => res.data);
}
