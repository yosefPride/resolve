import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createLink, deleteLink, listLinks } from '../services/links.service';

export function useLinks(groupId, ticketId) {
  return useQuery({
    queryKey: ['links', groupId, ticketId],
    queryFn: () => listLinks(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

export function useCreateLink(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input) => createLink(groupId, ticketId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['links', groupId, ticketId] });
    },
  });
}

export function useDeleteLink(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (linkId) => deleteLink(groupId, ticketId, linkId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['links', groupId, ticketId] });
    },
  });
}
