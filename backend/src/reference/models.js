import { datetime, model, objectId, option, string } from '../db/model.js';
import * as t from '../serde/types.js';

// An external URL a user attaches to a ticket (a PR, a doc, a Slack thread) —
// distinct from a ticket link, which relates two tickets. `label` is always
// populated: ReferenceService derives it from the URL's host when the caller
// leaves it blank.
export const decodeReference = model('TicketReference', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  label: string,
  url: string,
  created_by: objectId,
  created_at: datetime,
});

export const CreateReferenceRequest = t.struct('CreateReferenceRequest', {
  // Blank/absent falls back to the URL's host rather than being rejected.
  label: t.option(t.string),
  url: t.string,
});
