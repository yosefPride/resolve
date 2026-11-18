import { datetime, i64, model, objectId, oneOf, option, string } from '../db/model.js';
import * as t from '../serde/types.js';

export const TicketStatus = { Open: 'open', Closed: 'closed' };
export const TicketPriority = { Low: 'low', High: 'high', Critical: 'critical' };
const STATUSES = ['open', 'closed'];
const PRIORITIES = ['low', 'high', 'critical'];

// ticket_number is a running number scoped to group_id (1, 2, 3, ...),
// allocated atomically via TicketRepository.nextTicketNumber — the
// human-facing number, distinct from _id. content_updated_at is bumped only
// when title/description/priority change — NOT on a status-only change — and
// is what AI insights are compared against.
export const decodeTicket = model('Ticket', {
  _id: option(objectId),
  group_id: objectId,
  ticket_number: i64,
  title: string,
  description: string,
  status: oneOf(STATUSES),
  priority: oneOf(PRIORITIES),
  created_by: objectId,
  created_at: datetime,
  updated_at: datetime,
  content_updated_at: datetime,
});

// Backs the per-group ticket_number sequence; one document per group, keyed
// by group_id as _id.
export const decodeTicketCounter = model('TicketCounter', {
  _id: objectId,
  ticket_seq: i64,
});

const Status = t.unitEnum('TicketStatus', STATUSES);
const Priority = t.unitEnum('TicketPriority', PRIORITIES);

export const CreateTicketRequest = t.struct('CreateTicketRequest', {
  title: t.string,
  description: t.string,
  priority: Priority,
});

// All fields optional; the handler rejects a request where every field is absent.
export const UpdateTicketRequest = t.struct('UpdateTicketRequest', {
  title: t.option(t.string),
  description: t.option(t.string),
  priority: t.option(Priority),
  status: t.option(Status),
});

// From the query string, so every field is optional and `creator` stays a
// raw hex string (parsed in the service, like other ids).
export const ListTicketsQuery = t.struct('ListTicketsQuery', {
  q: t.option(t.string),
  status: t.option(Status),
  priority: t.option(Priority),
  creator: t.option(t.string),
  page: t.option(t.u64),
  per_page: t.option(t.u64),
});
