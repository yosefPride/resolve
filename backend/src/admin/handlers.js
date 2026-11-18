import { ApiError } from '../errors.js';
import { trim } from '../rust.js';
import { json, path, query } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { systemAdminUser } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { AdminListQuery, AuditLogQuery, DeleteUserRequest } from './models.js';
import { AdminService } from './service.js';

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

function searchTerm(listQuery) {
  const term = listQuery.search === null ? '' : trim(listQuery.search);
  return term === '' ? null : term;
}

export const deletionCheck = handler([systemAdminUser, path], async ({ state }, user, [rawId]) => {
  const targetId = parseId(rawId);
  const result = await new AdminService(state.db).deletionCheck(user.userId, targetId);
  return HttpResponse.ok().json(result);
});

export const deleteUser = handler(
  [systemAdminUser, path, json(DeleteUserRequest)],
  async ({ state }, user, [rawId], input) => {
    const targetId = parseId(rawId);
    const successors = new Map();
    for (const [groupId, successorId] of input.successors) {
      successors.set(parseId(groupId).toHexString(), parseId(successorId));
    }
    await new AdminService(state.db).deleteUser(user.userId, targetId, successors);
    return HttpResponse.noContent().finish();
  },
);

export const promoteUser = handler([systemAdminUser, path], async ({ state }, user, [rawId]) => {
  const targetId = parseId(rawId);
  await new AdminService(state.db).promoteUser(user.userId, targetId);
  return HttpResponse.noContent().finish();
});

export const demoteUser = handler([systemAdminUser, path], async ({ state }, user, [rawId]) => {
  const targetId = parseId(rawId);
  await new AdminService(state.db).demoteUser(user.userId, targetId);
  return HttpResponse.noContent().finish();
});

export const listUsers = handler(
  [systemAdminUser, query(AdminListQuery)],
  async ({ state }, user, listQuery) => {
    const users = await new AdminService(state.db).listUsers(user.userId, searchTerm(listQuery));
    return HttpResponse.ok().json(users);
  },
);

export const listGroups = handler(
  [systemAdminUser, query(AdminListQuery)],
  async ({ state }, user, listQuery) => {
    const groups = await new AdminService(state.db).listGroups(user.userId, searchTerm(listQuery));
    return HttpResponse.ok().json(groups);
  },
);

export const listAuditLog = handler(
  [systemAdminUser, query(AuditLogQuery)],
  async ({ state }, user, logQuery) => {
    const groupId = logQuery.group_id === null ? null : parseId(logQuery.group_id);
    const deletedUserId = logQuery.user_id === null ? null : parseId(logQuery.user_id);
    const entries = await new AdminService(state.db).listAuditLog(user.userId, groupId, deletedUserId);
    return HttpResponse.ok().json(entries);
  },
);

export const deleteGroup = handler([systemAdminUser, path], async ({ state }, user, [rawId]) => {
  const groupId = parseId(rawId);
  await new AdminService(state.db).deleteGroup(user.userId, groupId);
  return HttpResponse.noContent().finish();
});
