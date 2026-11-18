import { datetime, model, objectId, oneOf, option } from '../db/model.js';
import * as t from '../serde/types.js';

// The relation as stored — always from source_ticket_id's point of view.
// `relates_to` is symmetric: LinkService checks both directions before
// insert, so an A<->B pair is only ever one document. `blocks`/`duplicates`
// are directional; the inverse reading is derived at read time.
export const RelationType = { Blocks: 'blocks', RelatesTo: 'relates_to', Duplicates: 'duplicates' };
const RELATION_TYPES = Object.values(RelationType);

// The label a viewer should see, given which side of the stored relation
// their ticket is on.
export function labelFor(relationType, viewpointIsSource) {
  switch (relationType) {
    case RelationType.Blocks: return viewpointIsSource ? 'blocks' : 'is_blocked_by';
    case RelationType.RelatesTo: return 'relates_to';
    default: return viewpointIsSource ? 'duplicates' : 'is_duplicated_by';
  }
}

// Short human-readable text for the activity log.
export function describeLabel(label) {
  return {
    blocks: 'blocks',
    is_blocked_by: 'is blocked by',
    relates_to: 'relates to',
    duplicates: 'duplicates',
    is_duplicated_by: 'is duplicated by',
  }[label];
}

export const decodeLink = model('TicketLink', {
  _id: option(objectId),
  group_id: objectId,
  source_ticket_id: objectId,
  target_ticket_id: objectId,
  relation_type: oneOf(RELATION_TYPES),
  created_by: objectId,
  created_at: datetime,
});

export const CreateLinkRequest = t.struct('CreateLinkRequest', {
  target_ticket_id: t.string,
  relation_type: t.unitEnum('RelationType', RELATION_TYPES),
});
