import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

// Builds the list/create/delete hook triple shared by ticket-scoped
// collections (links, references): a list query keyed on
// [key, groupId, ticketId], and mutations that invalidate that key on
// success. Resources whose mutations must touch other query keys too
// (e.g. useTickets invalidating ['groups']) don't fit here — keep those
// hand-written.
export function makeTicketResourceHooks(key, { list, create, remove }) {
  function useList(groupId, ticketId) {
    return useQuery({
      queryKey: [key, groupId, ticketId],
      queryFn: () => list(groupId, ticketId),
      enabled: Boolean(groupId) && Boolean(ticketId),
    });
  }

  function useResourceMutation(groupId, ticketId, fn) {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (arg) => fn(groupId, ticketId, arg),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: [key, groupId, ticketId] });
      },
    });
  }

  return {
    useList,
    useCreate: (groupId, ticketId) => useResourceMutation(groupId, ticketId, create),
    useDelete: (groupId, ticketId) => useResourceMutation(groupId, ticketId, remove),
  };
}
