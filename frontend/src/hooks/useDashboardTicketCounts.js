import { useQueries } from '@tanstack/react-query';
import { listTickets } from '../services/tickets.service';

// Exact counts for the dashboard's Critical/High Open and My Open Issues
// tiles. Rather than fetching ticket rows and filtering them client-side
// (bounded by a page-size cap, so it undercounts for large groups — see
// useDashboardOverview), this reads `total` from TicketService::list_tickets,
// which is computed over the ENTIRE filtered set before the page/per_page
// slice (backend/src/ticket/service.rs). So per_page can stay at 1 — the
// smallest response the endpoint can return — and every count is exact
// regardless of group size.
const ALERT_PRIORITIES = ['critical', 'high'];

export function useDashboardTicketCounts(groups, userId) {
  const priorityResults = useQueries({
    queries: groups.flatMap((group) =>
      ALERT_PRIORITIES.map((priority) => ({
        queryKey: ['tickets', group.id, { status: 'open', priority, perPage: 1 }],
        queryFn: () => listTickets(group.id, { status: 'open', priority, perPage: 1 }),
        enabled: Boolean(group.id),
      })),
    ),
  });

  const mineResults = useQueries({
    queries: groups.map((group) => ({
      queryKey: ['tickets', group.id, { status: 'open', creator: userId, perPage: 1 }],
      queryFn: () => listTickets(group.id, { status: 'open', creator: userId, perPage: 1 }),
      enabled: Boolean(group.id) && Boolean(userId),
    })),
  });

  const isLoading =
    priorityResults.some((result) => result.status === 'pending') ||
    mineResults.some((result) => result.status === 'pending');

  const sumTotals = (results) =>
    results.reduce((sum, result) => sum + (result.data?.total ?? 0), 0);

  return {
    isLoading,
    criticalOrHighOpen: sumTotals(priorityResults),
    myOpenTickets: sumTotals(mineResults),
  };
}
