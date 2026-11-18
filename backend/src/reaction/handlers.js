import { ApiError } from '../errors.js';
import { charCount, trim } from '../rust.js';
import { json, path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { SetReactionRequest } from './models.js';
import { ReactionService } from './service.js';

// Generous relative to a real emoji: covers ZWJ sequences and
// skin-tone/flag modifiers without accepting an arbitrary sentence.
const MAX_EMOJI_CHARS = 8;

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

function validateEmoji(input) {
  const trimmed = trim(input.emoji);
  if (trimmed === '') throw ApiError.validation('emoji is required');
  if (charCount(trimmed) > MAX_EMOJI_CHARS) throw ApiError.validation('emoji is too long');
  return trimmed;
}

// groupScoped consumes {id}; the path still yields every segment, so the
// first is dropped in favor of scoped.groupId.
export const setReaction = handler(
  [groupScoped, path, json(SetReactionRequest)],
  async ({ state }, scoped, [, rawTicketId, rawCommentId], input) => {
    const ticketId = parseId(rawTicketId);
    const commentId = parseId(rawCommentId);
    const emoji = validateEmoji(input);
    const reactions = await new ReactionService(state.db).setReaction(
      scoped.userId,
      scoped.groupId,
      ticketId,
      commentId,
      emoji,
    );
    return HttpResponse.ok().json(reactions);
  },
);

export const removeReaction = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawCommentId]) => {
    const ticketId = parseId(rawTicketId);
    const commentId = parseId(rawCommentId);
    const reactions = await new ReactionService(state.db).removeReaction(
      scoped.userId,
      scoped.groupId,
      ticketId,
      commentId,
    );
    return HttpResponse.ok().json(reactions);
  },
);
