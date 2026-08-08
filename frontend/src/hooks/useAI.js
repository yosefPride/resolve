import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  analyzeTicket,
  clearChat,
  generateGroupReport,
  listChatMessages,
  sendChatMessage,
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

// Unlike the insight queries above, chat is a real conversation history, not
// a cache-backed derived value — it should load whenever the panel is open,
// same as useComments.
export function useChatMessages(groupId, ticketId) {
  return useQuery({
    queryKey: ['ai-chat', groupId, ticketId],
    queryFn: () => listChatMessages(groupId, ticketId),
    enabled: Boolean(groupId) && Boolean(ticketId),
  });
}

// Invalidates rather than splicing the response into the cache: the response
// carries both the user and assistant messages, and refetching the thread is
// simpler than reasoning about where two new entries land relative to
// whatever else might have changed it (e.g. a concurrent clear).
export function useSendChatMessage(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (message) => sendChatMessage(groupId, ticketId, message),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-chat', groupId, ticketId] });
    },
  });
}

export function useClearChat(groupId, ticketId) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => clearChat(groupId, ticketId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-chat', groupId, ticketId] });
    },
  });
}
