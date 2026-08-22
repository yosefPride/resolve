import { createReference, deleteReference, listReferences } from '../services/references.service';
import { makeTicketResourceHooks } from './ticketResourceHooks';

const hooks = makeTicketResourceHooks('references', {
  list: listReferences,
  create: createReference,
  remove: deleteReference,
});

export const useReferences = hooks.useList;
export const useCreateReference = hooks.useCreate;
export const useDeleteReference = hooks.useDelete;
