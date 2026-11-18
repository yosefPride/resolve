import { insertWithId } from '../utils.js';
import { decodeActivity } from './models.js';

// Shared infrastructure: most writes come from other services (ticket,
// comment, link, reference record events; group/admin cascade deletes) — the
// service in this module only ever reads.
export class ActivityRepository {
  constructor(db) {
    this.activity = db.collection('ticket_activity');
  }

  async insert({ groupId, ticketId, actorId, eventType, oldValue = null, newValue = null, commentId = null, linkKind = null }) {
    const entry = {
      group_id: groupId,
      ticket_id: ticketId,
      actor_id: actorId,
      event_type: eventType,
      old_value: oldValue,
      new_value: newValue,
      comment_id: commentId,
      link_kind: linkKind,
      occurred_at: new Date(),
    };
    const id = await insertWithId(this.activity, entry);
    return { _id: id, ...entry };
  }

  // Newest-first — a timeline reads most-recent-first.
  async listByTicket(groupId, ticketId) {
    const docs = await this.activity
      .find({ group_id: groupId, ticket_id: ticketId })
      .sort({ occurred_at: -1 })
      .toArray();
    return docs.map(decodeActivity);
  }

  // The single most recent entry across every ticket in the group — backs the
  // group list's "Last Activity" stat.
  async findLatestForGroup(groupId) {
    const doc = await this.activity.findOne({ group_id: groupId }, { sort: { occurred_at: -1 } });
    return doc ? decodeActivity(doc) : null;
  }

  // Cascade target for a single ticket's deletion.
  async deleteByTicket(groupId, ticketId) {
    return (await this.activity.deleteMany({ group_id: groupId, ticket_id: ticketId })).deletedCount;
  }

  // Cascade target for a whole group's deletion.
  async deleteByGroup(groupId) {
    return (await this.activity.deleteMany({ group_id: groupId })).deletedCount;
  }
}
