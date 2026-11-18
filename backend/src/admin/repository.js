import { insertWithId } from '../utils.js';
import { decodeAuditEntry } from './models.js';

export class AdminRepository {
  constructor(db) {
    this.auditLog = db.collection('admin_audit_log');
  }

  async insertAuditEntry(entry) {
    const id = await insertWithId(this.auditLog, entry);
    return { _id: id, ...entry };
  }

  // Newest-first. Each supplied filter narrows the result; both absent
  // returns the whole log.
  async listAuditLog(groupId, deletedUserId) {
    const filter = {};
    if (groupId !== null) filter.group_id = groupId;
    if (deletedUserId !== null) filter.deleted_user_id = deletedUserId;
    const docs = await this.auditLog.find(filter).sort({ created_at: -1 }).toArray();
    return docs.map(decodeAuditEntry);
  }
}
