import { useQueries } from '@tanstack/react-query';
import { listTickets } from '../services/tickets.service';

// One ticket page (up to the API's per-group max) per team, fetched in
// parallel — each request is a normal RBAC-scoped /groups/{id}/tickets call,
// merged client-side, never a cross-group query. From this single response
// per group the dashboard derives everything it needs (priority breakdown,
// "created by me" count, cross-team recent list) without a request per
// widget. Groups with more than 100 tickets undercount past the cap;
// acceptable for an overview widget, not a source of truth.
const OVERVIEW_FILTERS = { perPage: 100 };

export function useDashboardOverview(groups) {
  const results = useQueries({
    queries: groups.map((group) => ({
      queryKey: ['tickets', group.id, OVERVIEW_FILTERS],
      queryFn: () => listTickets(group.id, OVERVIEW_FILTERS),
      enabled: Boolean(group.id),
    })),
  });

  const isLoading = results.length > 0 && results.some((result) => result.status === 'pending');

  const tickets = results.flatMap((result, index) => {
    const group = groups[index];
    return (result.data?.items ?? []).map((ticket) => ({ ...ticket, group_name: group.name }));
  });

  return { isLoading, tickets };
}
