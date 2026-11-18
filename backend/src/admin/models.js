import { datetime, model, objectId, oneOf, option, string, withDefault } from '../db/model.js';
import { formatDateTime } from '../rust.js';
import * as t from '../serde/types.js';

export const AuditAction = {
  Succession: 'succession',
  GroupAutoDeleted: 'group_auto_deleted',
  Promotion: 'promotion',
  Demotion: 'demotion',
};

// One row per System Admin action worth an audit trail. Fields are
// action-specific: succession and group_auto_deleted fill the group and
// deleted-user fields (successor too, for succession only); promotion and
// demotion fill only the target-user fields. performed_by is on every row.
// Names are snapshotted at write time, since the users/groups involved may
// no longer exist when the log is read. Defaults let entries written before
// a field existed still be read.
export const decodeAuditEntry = model('AuditLogEntry', {
  _id: option(objectId),
  action: oneOf(Object.values(AuditAction)),
  group_id: option(objectId),
  group_name: withDefault(string, ''),
  deleted_user_id: option(objectId),
  deleted_user_name: withDefault(string, ''),
  successor_user_id: option(objectId),
  successor_user_name: option(string),
  target_user_id: option(objectId),
  target_user_name: option(string),
  performed_by: objectId,
  performed_by_name: withDefault(string, ''),
  created_at: datetime,
});

// A full audit row as stored: every field present, absent ones as null/"".
export function auditEntry(fields) {
  return {
    action: fields.action,
    group_id: fields.groupId ?? null,
    group_name: fields.groupName ?? '',
    deleted_user_id: fields.deletedUserId ?? null,
    deleted_user_name: fields.deletedUserName ?? '',
    successor_user_id: fields.successorUserId ?? null,
    successor_user_name: fields.successorUserName ?? null,
    target_user_id: fields.targetUserId ?? null,
    target_user_name: fields.targetUserName ?? null,
    performed_by: fields.performedBy,
    performed_by_name: fields.performedByName,
    created_at: new Date(),
  };
}

const hexOrNull = (id) => (id ? id.toHexString() : null);

export function auditEntryResponse(entry) {
  return {
    id: entry._id ? entry._id.toHexString() : '',
    action: entry.action,
    group_id: hexOrNull(entry.group_id),
    group_name: entry.group_name,
    deleted_user_id: hexOrNull(entry.deleted_user_id),
    deleted_user_name: entry.deleted_user_name,
    successor_user_id: hexOrNull(entry.successor_user_id),
    successor_user_name: entry.successor_user_name,
    target_user_id: hexOrNull(entry.target_user_id),
    target_user_name: entry.target_user_name,
    performed_by: entry.performed_by.toHexString(),
    performed_by_name: entry.performed_by_name,
    created_at: formatDateTime(entry.created_at),
  };
}

// Optional filters for GET /admin/audit-log; `user_id` filters on the
// *deleted* user.
export const AuditLogQuery = t.struct('AuditLogQuery', {
  group_id: t.option(t.string),
  user_id: t.option(t.string),
});

// GET /admin/users and /admin/groups: case-insensitive substring `search`.
export const AdminListQuery = t.struct('AdminListQuery', { search: t.option(t.string) });

// group_id (hex) -> successor user_id (hex). Only groups listed in a
// deletion check's blocked_groups need an entry.
export const DeleteUserRequest = t.struct('DeleteUserRequest', { successors: t.stringMap });
