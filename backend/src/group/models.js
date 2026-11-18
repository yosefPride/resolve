import { datetime, model, objectId, oneOf, option, string } from '../db/model.js';
import { formatDateTime } from '../rust.js';
import * as t from '../serde/types.js';

export const Role = { Contributor: 'contributor', GroupAdmin: 'group_admin' };
const ROLES = [Role.Contributor, Role.GroupAdmin];

export const decodeGroup = model('Group', {
  _id: option(objectId),
  name: string,
  owner_id: objectId,
  created_at: datetime,
});

export const decodeGroupMember = model('GroupMember', {
  _id: option(objectId),
  group_id: objectId,
  user_id: objectId,
  role: oneOf(ROLES),
  joined_at: datetime,
});

// GroupResponse: the plain group-metadata shape used for create/get/rename
// and admin's system-wide group list.
export function groupResponse(group) {
  return {
    id: group._id ? group._id.toHexString() : '',
    name: group.name,
    owner_id: group.owner_id.toHexString(),
    created_at: formatDateTime(group.created_at),
  };
}

// MemberResponse needs name/email, which GroupMember doesn't carry; a deleted
// account reads as empty strings.
export function memberResponse(member, user) {
  return {
    id: member._id ? member._id.toHexString() : '',
    user_id: member.user_id.toHexString(),
    name: user ? user.name : '',
    email: user ? user.email : '',
    role: member.role,
    joined_at: formatDateTime(member.joined_at),
  };
}

const RoleType = t.unitEnum('Role', ROLES);
export const CreateGroupRequest = t.struct('CreateGroupRequest', { name: t.string });
export const AddMemberRequest = t.struct('AddMemberRequest', { user_id: t.string, role: RoleType });
export const LookupUserQuery = t.struct('LookupUserQuery', { email: t.string });
export const UpdateMemberRoleRequest = t.struct('UpdateMemberRoleRequest', { role: RoleType });
