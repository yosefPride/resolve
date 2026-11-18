import { ApiError } from '../errors.js';
import { charCount, trim } from '../rust.js';
import { json, path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { CreateCommentRequest } from './models.js';
import { CommentService } from './service.js';

const MAX_CONTENT_LEN = 2000;

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

// Counted in characters, not UTF-8 bytes, so Hebrew/non-Latin content isn't
// rejected under the nominal limit.
function validateCreate(input) {
  if (trim(input.content) === '') throw ApiError.validation('content is required');
  if (charCount(input.content) > MAX_CONTENT_LEN) {
    throw ApiError.validation(`content must be at most ${MAX_CONTENT_LEN} characters`);
  }
}

export const createComment = handler(
  [groupScoped, path, json(CreateCommentRequest)],
  async ({ state }, scoped, [, rawTicketId], input) => {
    const ticketId = parseId(rawTicketId);
    validateCreate(input);
    const parentCommentId = input.parent_comment_id === null ? null : parseId(input.parent_comment_id);
    const comment = await new CommentService(state.db).createComment(
      scoped.userId,
      scoped.groupId,
      ticketId,
      input.content,
      parentCommentId,
    );
    return HttpResponse.created().json(comment);
  },
);

export const listComments = handler([groupScoped, path], async ({ state }, scoped, [, rawTicketId]) => {
  const ticketId = parseId(rawTicketId);
  const comments = await new CommentService(state.db).listComments(
    scoped.userId,
    scoped.groupId,
    ticketId,
  );
  return HttpResponse.ok().json(comments);
});

export const deleteComment = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawCommentId]) => {
    const ticketId = parseId(rawTicketId);
    const commentId = parseId(rawCommentId);
    await new CommentService(state.db).deleteComment(scoped.userId, scoped.groupId, ticketId, commentId);
    return HttpResponse.noContent().finish();
  },
);
