import { createLink, deleteLink, listLinks } from '../services/links.service';
import { makeTicketResourceHooks } from './ticketResourceHooks';

const hooks = makeTicketResourceHooks('links', {
  list: listLinks,
  create: createLink,
  remove: deleteLink,
});

export const useLinks = hooks.useList;
export const useCreateLink = hooks.useCreate;
export const useDeleteLink = hooks.useDelete;
