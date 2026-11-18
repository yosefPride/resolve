import { ApiError } from '../errors.js';
import { insertWithId, isDuplicateKey, substringRegex } from '../utils.js';
import { Role, decodeGroup, decodeGroupMember } from './models.js';

// Any duplicate-key failure here is the unique (group_id, user_id) member
// index (GroupRepoError::DuplicateMember).
async function guard(promise) {
  try {
    return await promise;
  } catch (err) {
    if (isDuplicateKey(err)) throw ApiError.conflict('user is already a member of this group');
    throw err;
  }
}

export class GroupRepository {
  constructor(db) {
    this.groups = db.collection('groups');
    this.members = db.collection('group_members');
  }

  async createGroup({ name, ownerId }) {
    const group = { name, owner_id: ownerId, created_at: new Date() };
    const id = await guard(insertWithId(this.groups, group));
    return { _id: id, ...group };
  }

  async findGroupById(id) {
    const doc = await guard(this.groups.findOne({ _id: id }));
    return doc ? decodeGroup(doc) : null;
  }

  // `search`, when present and non-empty, filters to groups whose name
  // contains it (case-insensitive substring). Absent/blank returns every group.
  async listAllGroups(search) {
    const filter = search ? { name: substringRegex(search) } : {};
    return (await guard(this.groups.find(filter).toArray())).map(decodeGroup);
  }

  async deleteGroup(id) {
    const result = await guard(this.groups.deleteOne({ _id: id }));
    return result.deletedCount > 0;
  }

  async renameGroup(id, name) {
    const result = await guard(this.groups.updateOne({ _id: id }, { $set: { name } }));
    return result.modifiedCount > 0;
  }

  async insertMember(groupId, userId, role) {
    const member = { group_id: groupId, user_id: userId, role, joined_at: new Date() };
    const id = await guard(insertWithId(this.members, member));
    return { _id: id, ...member };
  }

  async findMember(groupId, userId) {
    const doc = await guard(this.members.findOne({ group_id: groupId, user_id: userId }));
    return doc ? decodeGroupMember(doc) : null;
  }

  async listMembers(groupId) {
    return (await guard(this.members.find({ group_id: groupId }).toArray())).map(decodeGroupMember);
  }

  async listGroupsForUser(userId) {
    const memberships = (await guard(this.members.find({ user_id: userId }).toArray())).map(
      decodeGroupMember,
    );
    if (memberships.length === 0) return [];
    const groupIds = memberships.map((m) => m.group_id);
    return (await guard(this.groups.find({ _id: { $in: groupIds } }).toArray())).map(decodeGroup);
  }

  // Same "user_id" query as listGroupsForUser, but keeps the membership (and
  // its role) — used by listMyGroups, which needs the caller's role per group.
  async listMembershipsForUser(userId) {
    return (await guard(this.members.find({ user_id: userId }).toArray())).map(decodeGroupMember);
  }

  countMembers(groupId) {
    return guard(this.members.countDocuments({ group_id: groupId }));
  }

  countGroupAdmins(groupId) {
    return guard(this.members.countDocuments({ group_id: groupId, role: Role.GroupAdmin }));
  }

  async updateMemberRole(groupId, userId, role) {
    const result = await guard(
      this.members.updateOne({ group_id: groupId, user_id: userId }, { $set: { role } }),
    );
    return result.modifiedCount > 0;
  }

  async deleteMember(groupId, userId) {
    const result = await guard(this.members.deleteOne({ group_id: groupId, user_id: userId }));
    return result.deletedCount > 0;
  }

  async deleteMembersByGroup(groupId) {
    const result = await guard(this.members.deleteMany({ group_id: groupId }));
    return result.deletedCount;
  }
}
