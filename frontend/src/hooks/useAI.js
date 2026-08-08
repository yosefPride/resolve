import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  analyzeTicket,
  createConversation,
  deleteConversation,
  generateGroupReport,
  listConversationMessages,
  listConversations,
  sendConversationMessage,
  summarizeTicket,
} from '../services/ai.service';

// These endpoints are idempotent, cache-backed reads on the backend (a hit
// returns the stored insight/report without calling Gemini again), not
// mutations — so they're modeled as queries with enabled: false rather than
// useMutation. Nothing fetches on mount or ticket change; the caller invokes
// refetch() from a button click. Caching then falls out of the query key for
// free: revisiting an already-fetched ticket in this session shows the
// result instantly, switching to a different ticket shows the empty state.
// retry is disabled so a failure doesn't trigger a second unwanted Gemini call.

export function useTicketSummary(groupId, ticketId) {
  return useQuery({
    queryKey: ['ai-summary', groupId, ticketId],
    queryFn: () => summarizeTicket(groupId, ticketId),
    enabled: false,
    retry: false,
  });
}

export function useTicketAnalysis(groupId, ticketId) {
  return useQuery({
    queryKey: ['ai-analysis', groupId, ticketId],
    queryFn: () => analyzeTicket(groupId, ticketId),
    enabled: false,
    retry: false,
  });
}

export function useGroupReport(groupId) {
  return useQuery({
    queryKey: ['ai-report', groupId],
    queryFn: () => generateGroupReport(groupId),
    enabled: false,
    retry: false,
  });
}

// The caller's own conversations for this ticket, most-recently-active
// first (backend-sorted) — a real list, not a cache-backed derived value, so
// it loads whenever the panel is open, same as useComments.
export function useConversations(groupId, ticketId) {
  return useQuery({
    queryKey: ['ai-conversations', groupId, ticketId],
    queryFn: () => listConversations(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

export function useCreateConversation(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => createConversation(groupId, ticketId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-conversations', groupId, ticketId] });
    },
  });
}

// enabled: Boolean(conversationId) — no request until a conversation is
// actually selected, which is what makes "no active conversation" a free,
// query-less state rather than something the caller has to special-case.
export function useConversationMessages(groupId, ticketId, conversationId) {
  return useQuery({
    queryKey: ['ai-conversation-messages', groupId, ticketId, conversationId],
    queryFn: () => listConversationMessages(groupId, ticketId, conversationId),
    enabled: Boolean(groupId) && Boolean(ticketId) && Boolean(conversationId),
  });
}

// Takes conversationId as a mutate-time argument (not fixed at hook-creation
// time, same reasoning as useDeleteConversation below) — a message sent right
// after auto-creating a conversation needs to target the id that create just
// returned, before the "active conversation" prop has re-rendered with it.
//
// Invalidates rather than splicing the response into the cache: the response
// carries both the user and assistant messages, and refetching the
// conversation is simpler than reasoning about where two new entries land.
// Also invalidates the conversations list: sending sets the title on a
// conversation's first message and always bumps its updated_at (list order).
export function useSendConversationMessage(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ conversationId, message }) =>
      sendConversationMessage(groupId, ticketId, conversationId, message),
    onSuccess: (_data, { conversationId }) => {
      queryClient.invalidateQueries({
        queryKey: ['ai-conversation-messages', groupId, ticketId, conversationId],
      });
      queryClient.invalidateQueries({ queryKey: ['ai-conversations', groupId, ticketId] });
    },
  });
}

// Takes conversationId as the mutate argument (not fixed at hook-creation
// time) since the switcher can delete any conversation in the list, not just
// whichever one happens to be active.
export function useDeleteConversation(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (conversationId) => deleteConversation(groupId, ticketId, conversationId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-conversations', groupId, ticketId] });
    },
  });
}
