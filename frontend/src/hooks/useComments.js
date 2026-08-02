import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createComment, deleteComment, listComments } from '../services/comments.service';

export function useComments(groupId, ticketId) {
  return useQuery({
    queryKey: ['comments', groupId, ticketId],
    queryFn: () => listComments(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

// Unlike useTickets, nothing here touches ['groups']: a comment doesn't change
// open_ticket_count, so the sidebar/GroupStats numbers can't go stale from one.
export function useCreateComment(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input) => createComment(groupId, ticketId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['comments', groupId, ticketId] });
    },
  });
}

// Refetches rather than dropping the comment from the cache: the backend picks
// between a hard delete (leaf comment) and a tombstone (comment with replies,
// which stays in the list with is_deleted set), and reproducing that rule here
// to guess the outcome would just be a second copy of it to keep in sync.
export function useDeleteComment(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (commentId) => deleteComment(groupId, ticketId, commentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['comments', groupId, ticketId] });
    },
  });
}
