import { useQuery } from '@tanstack/react-query';
import { analyzeTicket, generateGroupReport, summarizeTicket } from '../services/ai.service';

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
