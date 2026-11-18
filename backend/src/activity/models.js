import { datetime, model, objectId, oneOf, option, string } from '../db/model.js';
import { formatDateTime } from '../rust.js';

// One entry per discrete change — an update touching status, priority, and
// title in a single PATCH writes three of these, so the Activity tab reads as
// granular events. Read-only from the API's point of view: entries are written
// only as a side effect of other modules' mutations.
export const ActivityEventType = {
  TicketCreated: 'ticket_created',
  StatusChanged: 'status_changed',
  PriorityChanged: 'priority_changed',
  TitleChanged: 'title_changed',
  DescriptionChanged: 'description_changed',
  CommentAdded: 'comment_added',
  CommentDeleted: 'comment_deleted',
  LinkAdded: 'link_added',
  LinkRemoved: 'link_removed',
};

// What a link_added/link_removed event refers to: a ticket-to-ticket relation
// (link module) or an external URL (reference module).
export const LinkKind = { Relation: 'relation', Reference: 'reference' };

// old_value/new_value: only for status/priority/title/link events (never
// description — long text isn't useful in a timeline). comment_id: only for
// comment events; no comment content is ever copied here. link_kind: only for
// link events.
export const decodeActivity = model('TicketActivity', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  actor_id: objectId,
  event_type: oneOf(Object.values(ActivityEventType)),
  old_value: option(string),
  new_value: option(string),
  comment_id: option(objectId),
  link_kind: option(oneOf(Object.values(LinkKind))),
  occurred_at: datetime,
});

export function activityResponse(entry, actor) {
  return {
    id: entry._id ? entry._id.toHexString() : '',
    group_id: entry.group_id.toHexString(),
    ticket_id: entry.ticket_id.toHexString(),
    actor_id: entry.actor_id.toHexString(),
    actor_name: actor ? actor.name : '',
    event_type: entry.event_type,
    old_value: entry.old_value,
    new_value: entry.new_value,
    comment_id: entry.comment_id ? entry.comment_id.toHexString() : null,
    link_kind: entry.link_kind,
    occurred_at: formatDateTime(entry.occurred_at),
  };
}
