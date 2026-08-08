import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createReference, deleteReference, listReferences } from '../services/references.service';

export function useReferences(groupId, ticketId) {
  return useQuery({
    queryKey: ['references', groupId, ticketId],
    queryFn: () => listReferences(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

export function useCreateReference(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input) => createReference(groupId, ticketId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['references', groupId, ticketId] });
    },
  });
}

export function useDeleteReference(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (referenceId) => deleteReference(groupId, ticketId, referenceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['references', groupId, ticketId] });
    },
  });
}
