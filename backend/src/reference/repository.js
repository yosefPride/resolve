import { insertWithId } from '../utils.js';
import { decodeReference } from './models.js';

export class ReferenceRepository {
  constructor(db) {
    this.references = db.collection('ticket_references');
  }

  async insert({ groupId, ticketId, label, url, createdBy }) {
    const reference = {
      group_id: groupId,
      ticket_id: ticketId,
      label,
      url,
      created_by: createdBy,
      created_at: new Date(),
    };
    const id = await insertWithId(this.references, reference);
    return { _id: id, ...reference };
  }

  async findById(groupId, ticketId, referenceId) {
    const doc = await this.references.findOne({ _id: referenceId, group_id: groupId, ticket_id: ticketId });
    return doc ? decodeReference(doc) : null;
  }

  // Oldest first.
  async listByTicket(groupId, ticketId) {
    const docs = await this.references
      .find({ group_id: groupId, ticket_id: ticketId })
      .sort({ created_at: 1 })
      .toArray();
    return docs.map(decodeReference);
  }

  async delete(groupId, ticketId, referenceId) {
    const result = await this.references.deleteOne({ _id: referenceId, group_id: groupId, ticket_id: ticketId });
    return result.deletedCount > 0;
  }

  // Cascade target for a single ticket's deletion.
  async deleteByTicket(groupId, ticketId) {
    return (await this.references.deleteMany({ group_id: groupId, ticket_id: ticketId })).deletedCount;
  }

  // Cascade target for a whole group's deletion.
  async deleteByGroup(groupId) {
    return (await this.references.deleteMany({ group_id: groupId })).deletedCount;
  }
}
