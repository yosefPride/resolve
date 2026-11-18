import { Long } from 'mongodb';

import { insertWithId } from '../utils.js';
import { TicketStatus, decodeTicket, decodeTicketCounter } from './models.js';

export class TicketRepository {
  constructor(db) {
    this.tickets = db.collection('tickets');
    this.counters = db.collection('counters');
  }

  // Atomically allocates the next ticket_number for a group: upsert + $inc
  // means two tickets created in the same group at the same instant still get
  // distinct numbers, with no check-then-insert race. Stored as Int64, like
  // Rust's i64.
  async nextTicketNumber(groupId) {
    const doc = await this.counters.findOneAndUpdate(
      { _id: groupId },
      { $inc: { ticket_seq: Long.fromInt(1) } },
      { upsert: true, returnDocument: 'after' },
    );
    return decodeTicketCounter(doc).ticket_seq;
  }

  async insertTicket(input) {
    const now = new Date();
    const ticket = {
      group_id: input.groupId,
      ticket_number: Long.fromNumber(input.ticketNumber),
      title: input.title,
      description: input.description,
      status: TicketStatus.Open,
      priority: input.priority,
      created_by: input.createdBy,
      created_at: now,
      updated_at: now,
      content_updated_at: now,
    };
    const id = await insertWithId(this.tickets, ticket);
    return { _id: id, ...ticket, ticket_number: input.ticketNumber };
  }

  // Filtered on group_id as well as _id: a ticket_id from one group stays
  // unreadable through another group's id (a mismatched pair finds nothing).
  async findById(groupId, ticketId) {
    const doc = await this.tickets.findOne({ _id: ticketId, group_id: groupId });
    return doc ? decodeTicket(doc) : null;
  }

  // status/priority/creator are exact-match, indexable fields, filtered here;
  // free-text title search is done in-process by the service.
  async listByGroup(groupId, status, priority, creator) {
    const filter = { group_id: groupId };
    if (status !== null) filter.status = status;
    if (priority !== null) filter.priority = priority;
    if (creator !== null) filter.created_by = creator;
    return (await this.tickets.find(filter).toArray()).map(decodeTicket);
  }

  async updateTicket(groupId, ticketId, changes) {
    const doc = await this.tickets.findOneAndUpdate(
      { _id: ticketId, group_id: groupId },
      { $set: changes },
      { returnDocument: 'after' },
    );
    return doc ? decodeTicket(doc) : null;
  }

  async deleteTicket(groupId, ticketId) {
    const result = await this.tickets.deleteOne({ _id: ticketId, group_id: groupId });
    return result.deletedCount > 0;
  }

  // Cascade target for group deletion: every ticket of the group plus the
  // counter that backs its ticket_number sequence (whose _id is the group_id).
  async deleteByGroup(groupId) {
    const result = await this.tickets.deleteMany({ group_id: groupId });
    await this.counters.deleteOne({ _id: groupId });
    return result.deletedCount;
  }

  countOpenByGroup(groupId) {
    return this.tickets.countDocuments({ group_id: groupId, status: TicketStatus.Open });
  }
}
