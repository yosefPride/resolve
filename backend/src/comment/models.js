import { bool, datetime, model, objectId, option, string } from '../db/model.js';
import * as t from '../serde/types.js';

// Content is replaced with DELETED_CONTENT_PLACEHOLDER and is_deleted flips to
// true when a comment with existing replies is deleted — the document stays so
// those replies keep a valid parent_comment_id. A leaf comment (no replies) is
// hard-deleted instead of tombstoned.
export const decodeComment = model('Comment', {
  _id: option(objectId),
  group_id: objectId,
  ticket_id: objectId,
  parent_comment_id: option(objectId),
  user_id: objectId,
  content: string,
  is_deleted: bool,
  created_at: datetime,
});

export const DELETED_CONTENT_PLACEHOLDER = '[comment deleted]';

export const CreateCommentRequest = t.struct('CreateCommentRequest', {
  content: t.string,
  parent_comment_id: t.option(t.string),
});
