import { ApiError } from '../errors.js';
import { byteLength, trim } from '../rust.js';
import { json, path, query } from '../server/extract.js';
import { handler } from '../server/handler.js';
import { groupScoped } from '../server/middleware.js';
import { HttpResponse } from '../server/response.js';
import { parseObjectId } from '../utils.js';
import { CreateTicketRequest, ListTicketsQuery, UpdateTicketRequest } from './models.js';
import { TicketService } from './service.js';

// Measured in UTF-8 bytes, as the Rust server did (str::len).
const MAX_TITLE_LEN = 200;

function parseId(raw) {
  const id = parseObjectId(raw);
  if (!id) throw ApiError.validation('invalid id');
  return id;
}

function validateCreate(input) {
  if (trim(input.title) === '') throw ApiError.validation('title is required');
  if (byteLength(input.title) > MAX_TITLE_LEN) {
    throw ApiError.validation(`title must be at most ${MAX_TITLE_LEN} characters`);
  }
  if (trim(input.description) === '') throw ApiError.validation('description is required');
}

function validateUpdate(input) {
  if (
    input.title === null &&
    input.description === null &&
    input.priority === null &&
    input.status === null
  ) {
    throw ApiError.validation('at least one field is required');
  }
  if (input.title !== null) {
    if (trim(input.title) === '') throw ApiError.validation('title must not be empty');
    if (byteLength(input.title) > MAX_TITLE_LEN) {
      throw ApiError.validation(`title must be at most ${MAX_TITLE_LEN} characters`);
    }
  }
  if (input.description !== null && trim(input.description) === '') {
    throw ApiError.validation('description must not be empty');
  }
}

export const createTicket = handler(
  [groupScoped, json(CreateTicketRequest)],
  async ({ state }, scoped, input) => {
    validateCreate(input);
    const ticket = await new TicketService(state.db).createTicket(
      scoped.userId,
      scoped.groupId,
      input,
    );
    return HttpResponse.created().json(ticket);
  },
);

export const listTickets = handler(
  [groupScoped, query(ListTicketsQuery)],
  async ({ state }, scoped, listQuery) => {
    const body = await new TicketService(state.db).listTickets(
      scoped.userId,
      scoped.groupId,
      listQuery,
    );
    return HttpResponse.ok().json(null, body);
  },
);

// groupScoped consumes the {id} segment; the path still yields both segments,
// so the first is dropped in favor of scoped.groupId.
export const getTicket = handler([groupScoped, path], async ({ state }, scoped, [, rawId]) => {
  const ticketId = parseId(rawId);
  const ticket = await new TicketService(state.db).getTicket(scoped.userId, scoped.groupId, ticketId);
  return HttpResponse.ok().json(ticket);
});

export const updateTicket = handler(
  [groupScoped, path, json(UpdateTicketRequest)],
  async ({ state }, scoped, [, rawId], input) => {
    const ticketId = parseId(rawId);
    validateUpdate(input);
    const ticket = await new TicketService(state.db).updateTicket(
      scoped.userId,
      scoped.groupId,
      ticketId,
      input,
    );
    return HttpResponse.ok().json(ticket);
  },
);

export const deleteTicket = handler([groupScoped, path], async ({ state }, scoped, [, rawId]) => {
  const ticketId = parseId(rawId);
  await new TicketService(state.db).deleteTicket(scoped.userId, scoped.groupId, ticketId);
  return HttpResponse.noContent().finish();
});
