import { ApiError } from '../errors.js';
import { charCount, trim } from '../rust.js';
import { json, path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { CreateReferenceRequest } from './models.js';
import { ReferenceService } from './service.js';

const MAX_URL_LEN = 2000;
const MAX_LABEL_LEN = 200;

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

// No URL-parsing library — only a scheme check and a length cap: this is a
// link a human will click, so "looks like an http(s) URL" is enough.
function validateCreate(input) {
  const url = trim(input.url);
  if (url === '') throw ApiError.validation('url is required');
  if (!(url.startsWith('http://') || url.startsWith('https://'))) {
    throw ApiError.validation('url must start with http:// or https://');
  }
  if (charCount(url) > MAX_URL_LEN) {
    throw ApiError.validation(`url must be at most ${MAX_URL_LEN} characters`);
  }
  if (input.label !== null && charCount(input.label) > MAX_LABEL_LEN) {
    throw ApiError.validation(`label must be at most ${MAX_LABEL_LEN} characters`);
  }
}

export const createReference = handler(
  [groupScoped, path, json(CreateReferenceRequest)],
  async ({ state }, scoped, [, rawTicketId], input) => {
    const ticketId = parseId(rawTicketId);
    validateCreate(input);
    const reference = await new ReferenceService(state.db).createReference(
      scoped.userId,
      scoped.groupId,
      ticketId,
      input,
    );
    return HttpResponse.created().json(reference);
  },
);

export const listReferences = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId]) => {
    const ticketId = parseId(rawTicketId);
    const references = await new ReferenceService(state.db).listReferences(
      scoped.userId,
      scoped.groupId,
      ticketId,
    );
    return HttpResponse.ok().json(references);
  },
);

export const deleteReference = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawReferenceId]) => {
    const ticketId = parseId(rawTicketId);
    const referenceId = parseId(rawReferenceId);
    await new ReferenceService(state.db).deleteReference(
      scoped.userId,
      scoped.groupId,
      ticketId,
      referenceId,
    );
    return HttpResponse.noContent().finish();
  },
);
