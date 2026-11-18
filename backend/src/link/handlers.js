import { ApiError } from '../errors.js';
import { json, path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { CreateLinkRequest } from './models.js';
import { LinkService } from './service.js';

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

export const createLink = handler(
  [groupScoped, path, json(CreateLinkRequest)],
  async ({ state }, scoped, [, rawTicketId], input) => {
    const ticketId = parseId(rawTicketId);
    const link = await new LinkService(state.db).createLink(scoped.userId, scoped.groupId, ticketId, input);
    return HttpResponse.created().json(link);
  },
);

export const listLinks = handler([groupScoped, path], async ({ state }, scoped, [, rawTicketId]) => {
  const ticketId = parseId(rawTicketId);
  const links = await new LinkService(state.db).listLinks(scoped.userId, scoped.groupId, ticketId);
  return HttpResponse.ok().json(links);
});

export const deleteLink = handler(
  [groupScoped, path],
  async ({ state }, scoped, [, rawTicketId, rawLinkId]) => {
    const ticketId = parseId(rawTicketId);
    const linkId = parseId(rawLinkId);
    await new LinkService(state.db).deleteLink(scoped.userId, scoped.groupId, ticketId, linkId);
    return HttpResponse.noContent().finish();
  },
);
