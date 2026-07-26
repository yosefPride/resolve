import { useMutation, useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import {
  createTicket,
  deleteTicket,
  getTicket,
  listTickets,
  updateTicket,
} from '../services/tickets.service';

// keepPreviousData keeps the current rows on screen while a filter/page change
// fetches, same as admin/UsersPanel's search — typing/paging swaps results in
// place instead of flashing a loading state.
export function useTicketList(groupId, filters) {
  return useQuery({
    queryKey: ['tickets', groupId, filters],
    queryFn: () => listTickets(groupId, filters),
    enabled: Boolean(groupId),
    placeholderData: keepPreviousData,
  });
}

export function useTicket(groupId, ticketId) {
  return useQuery({
    queryKey: ['ticket', groupId, ticketId],
    queryFn: () => getTicket(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

// Creating/updating/deleting a ticket also changes a group's
// open_ticket_count (GET /groups, shown in the sidebar and GroupStats), so
// every mutation here invalidates ['groups'] alongside the ticket keys it owns.
export function useCreateTicket(groupId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input) => createTicket(groupId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tickets', groupId] });
      queryClient.invalidateQueries({ queryKey: ['groups'] });
    },
  });
}

export function useUpdateTicket(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (changes) => updateTicket(groupId, ticketId, changes),
    onSuccess: (ticket) => {
      queryClient.setQueryData(['ticket', groupId, ticketId], ticket);
      queryClient.invalidateQueries({ queryKey: ['tickets', groupId] });
      queryClient.invalidateQueries({ queryKey: ['groups'] });
    },
  });
}

export function useDeleteTicket(groupId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (ticketId) => deleteTicket(groupId, ticketId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tickets', groupId] });
      queryClient.invalidateQueries({ queryKey: ['groups'] });
    },
  });
}
