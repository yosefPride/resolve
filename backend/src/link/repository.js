import { ApiError } from '../errors.js';
import { insertWithId, isDuplicateKey } from '../utils.js';
import { decodeLink } from './models.js';

// Any duplicate-key failure here is the unique link index (LinkRepoError::Duplicate).
async function guard(promise) {
  try {
    return await promise;
  } catch (err) {
    if (isDuplicateKey(err)) throw ApiError.conflict('this link already exists');
    throw err;
  }
}

export class LinkRepository {
  constructor(db) {
    this.links = db.collection('ticket_links');
  }

  async insert({ groupId, sourceTicketId, targetTicketId, relationType, createdBy }) {
    const link = {
      group_id: groupId,
      source_ticket_id: sourceTicketId,
      target_ticket_id: targetTicketId,
      relation_type: relationType,
      created_by: createdBy,
      created_at: new Date(),
    };
    const id = await guard(insertWithId(this.links, link));
    return { _id: id, ...link };
  }

  async findById(groupId, linkId) {
    const doc = await guard(this.links.findOne({ _id: linkId, group_id: groupId }));
    return doc ? decodeLink(doc) : null;
  }

  // Every link touching this ticket, whichever side it's stored on, oldest first.
  async listByTicket(groupId, ticketId) {
    const docs = await guard(
      this.links
        .find({
          group_id: groupId,
          $or: [{ source_ticket_id: ticketId }, { target_ticket_id: ticketId }],
        })
        .sort({ created_at: 1 })
        .toArray(),
    );
    return docs.map(decodeLink);
  }

  // Pre-insert duplicate check — the unique index is the actual race guard;
  // this just turns the common case into a clean Conflict.
  async exists(groupId, sourceTicketId, targetTicketId, relationType) {
    const count = await guard(
      this.links.countDocuments({
        group_id: groupId,
        source_ticket_id: sourceTicketId,
        target_ticket_id: targetTicketId,
        relation_type: relationType,
      }),
    );
    return count > 0;
  }

  async delete(groupId, linkId) {
    return (await guard(this.links.deleteOne({ _id: linkId, group_id: groupId }))).deletedCount > 0;
  }

  // Cascade target for a single ticket's deletion — removes every link where
  // the ticket appears on either side.
  async deleteByTicket(groupId, ticketId) {
    const result = await guard(
      this.links.deleteMany({
        group_id: groupId,
        $or: [{ source_ticket_id: ticketId }, { target_ticket_id: ticketId }],
      }),
    );
    return result.deletedCount;
  }

  // Cascade target for a whole group's deletion.
  async deleteByGroup(groupId) {
    return (await guard(this.links.deleteMany({ group_id: groupId }))).deletedCount;
  }
}
