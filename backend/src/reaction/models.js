import { datetime, model, objectId, option, string } from '../db/model.js';
import * as t from '../serde/types.js';

// One reaction per (comment_id, user_id) — enforced by the unique index and by
// ReactionRepository.setReaction's upsert on that key. Picking a new emoji
// replaces the user's existing reaction on the comment.
export const decodeReaction = model('CommentReaction', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  comment_id: objectId,
  user_id: objectId,
  emoji: string,
  created_at: datetime,
});

export const SetReactionRequest = t.struct('SetReactionRequest', { emoji: t.string });
