import { ApiError } from '../errors.js';
import { trim } from '../rust.js';
import { json, path, query } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { authenticatedUser, groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import {
  AddMemberRequest,
  CreateGroupRequest,
  LookupUserQuery,
  UpdateMemberRoleRequest,
} from './models.js';
import { GroupService } from './service.js';

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

function validateName(name) {
  if (trim(name) === '') throw ApiError.validation('name is required');
}

export const createGroup = handler(
  [authenticatedUser, json(CreateGroupRequest)],
  async ({ state }, user, input) => {
    validateName(input.name);
    const group = await new GroupService(state.db).createGroup(user.userId, input.name);
    return HttpResponse.created().json(group);
  },
);

export const listMyGroups = handler([authenticatedUser], async ({ state }, user) => {
  const groups = await new GroupService(state.db).listMyGroups(user.userId);
  return HttpResponse.ok().json(groups);
});

// The {id}-scoped handlers below take groupScoped rather than
// authenticatedUser: it authenticates, parses the {id} path segment, and
// verifies membership before the handler runs. The services still re-run
// their own role checks underneath.

export const getGroup = handler([groupScoped], async ({ state }, scoped) => {
  const group = await new GroupService(state.db).getGroup(scoped.userId, scoped.groupId);
  return HttpResponse.ok().json(group);
});

export const renameGroup = handler(
  [groupScoped, json(CreateGroupRequest)],
  async ({ state }, scoped, input) => {
    validateName(input.name);
    const group = await new GroupService(state.db).renameGroup(
      scoped.userId,
      scoped.groupId,
      input.name,
    );
    return HttpResponse.ok().json(group);
  },
);

export const deleteGroup = handler([groupScoped], async ({ state }, scoped) => {
  await new GroupService(state.db).deleteGroup(scoped.userId, scoped.groupId);
  return HttpResponse.noContent().finish();
});

export const listMembers = handler([groupScoped], async ({ state }, scoped) => {
  const members = await new GroupService(state.db).listMembers(scoped.userId, scoped.groupId);
  return HttpResponse.ok().json(members);
});

export const lookupUser = handler(
  [groupScoped, query(LookupUserQuery)],
  async ({ state }, scoped, { email }) => {
    if (trim(email) === '') throw ApiError.validation('email is required');
    const result = await new GroupService(state.db).lookupUserByEmail(
      scoped.userId,
      scoped.groupId,
      email,
    );
    return HttpResponse.ok().json(result);
  },
);

export const addMember = handler(
  [groupScoped, json(AddMemberRequest)],
  async ({ state }, scoped, input) => {
    const targetUserId = parseId(input.user_id);
    const member = await new GroupService(state.db).addMember(
      scoped.userId,
      scoped.groupId,
      targetUserId,
      input.role,
    );
    return HttpResponse.created().json(member);
  },
);

// groupScoped consumes the {id} segment; the path still yields both segments,
// so the first is dropped in favor of scoped.groupId.
export const updateMemberRole = handler(
  [groupScoped, path, json(UpdateMemberRoleRequest)],
  async ({ state }, scoped, [, targetUserIdRaw], input) => {
    const targetUserId = parseId(targetUserIdRaw);
    const member = await new GroupService(state.db).updateMemberRole(
      scoped.userId,
      scoped.groupId,
      targetUserId,
      input.role,
    );
    return HttpResponse.ok().json(member);
  },
);

// A single endpoint covers both admin-driven removal and self-service
// leaving: removing yourself doesn't require being a Group Admin, removing
// someone else does. Both paths still enforce the sole-Group-Admin guard.
export const removeMember = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, targetUserIdRaw]) => {
    const targetUserId = parseId(targetUserIdRaw);
    const service = new GroupService(state.db);
    if (targetUserId.equals(scoped.userId)) {
      await service.leaveGroup(scoped.userId, scoped.groupId);
    } else {
      await service.removeMember(scoped.userId, scoped.groupId, targetUserId);
    }
    return HttpResponse.noContent().finish();
  },
);
