import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { listGroups } from '../../services/groups.service';
import { useDashboardOverview } from '../../hooks/useDashboardOverview';
import Badge from '../../components/ui/Badge';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../tickets/badgeVariants';

const RECENT_LIMIT = 6;

// Shares the ['groups'] and ['tickets', groupId, ...] query keys with
// DashboardStats, so on a normal dashboard load this renders off the same
// cached data DashboardStats already fetched rather than firing its own
// round trips.
export default function RecentTickets() {
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });
  const { tickets, isLoading: ticketsLoading } = useDashboardOverview(groups);

  if (status !== 'success' || groups.length === 0) {
    return null;
  }

  if (ticketsLoading) {
    return <p className="text-sm text-slate-400">Loading recent activity…</p>;
  }

  if (tickets.length === 0) {
    return null;
  }

  const recent = [...tickets]
    .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at))
    .slice(0, RECENT_LIMIT);

  return (
    <div className="flex flex-col gap-3 border-t border-white/10 pt-6">
      <h2 className="text-sm font-semibold text-white">Recent activity</h2>
      <div className="flex flex-col gap-2">
        {recent.map((ticket) => (
          <Link
            key={ticket.id}
            to={`/tickets/${ticket.id}?group=${ticket.group_id}`}
            className="flex items-center justify-between gap-4 rounded-lg border border-white/10 bg-white/5 px-4 py-3 transition-colors hover:bg-white/10"
          >
            <div className="flex min-w-0 items-center gap-3">
              <span className="shrink-0 text-xs font-medium text-slate-500">
                #{ticket.ticket_number}
              </span>
              <span className="truncate text-sm font-medium text-white">{ticket.title}</span>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <span className="hidden text-xs text-slate-400 md:inline">{ticket.group_name}</span>
              <Badge variant={STATUS_VARIANT[ticket.status]}>{capitalize(ticket.status)}</Badge>
              <Badge variant={PRIORITY_VARIANT[ticket.priority]}>{capitalize(ticket.priority)}</Badge>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
