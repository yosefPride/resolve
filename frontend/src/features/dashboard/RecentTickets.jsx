import { useQuery } from '@tanstack/react-query';
import { listGroups } from '../../services/groups.service';
import { useDashboardOverview } from '../../hooks/useDashboardOverview';
import TicketCard from '../tickets/TicketCard';

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
          <TicketCard
            key={ticket.id}
            ticket={ticket}
            groupId={ticket.group_id}
            meta={ticket.group_name}
          />
        ))}
      </div>
    </div>
  );
}
