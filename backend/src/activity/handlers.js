import { ApiError } from '../errors.js';
import { path } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { ActivityService } from './service.js';

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

export const listActivity = handler([groupScoped, path], async ({ state }, scoped, [, rawId]) => {
  const ticketId = parseId(rawId);
  const activity = await new ActivityService(state.db).listActivity(
    scoped.userId,
    scoped.groupId,
    ticketId,
  );
  return HttpResponse.ok().json(activity);
});
