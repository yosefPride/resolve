import { useQuery } from '@tanstack/react-query';
import { Clock, Ticket, User } from 'lucide-react';
import { listGroups } from '../../services/groups.service';

function Stat({ icon: Icon, label, value }) {
  return (
    <div className="flex flex-col gap-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3">
      <span className="flex items-center gap-2 text-xs font-medium tracking-wide text-slate-400 uppercase">
        <Icon className="h-4 w-4" />
        {label}
      </span>
      <span className="text-lg font-semibold text-white">{value}</span>
    </div>
  );
}

// Open issues come from GET /groups, which reports the count for every team the
// caller belongs to — and viewing a team requires membership, so this team is
// always in that list. Same ['groups'] key the sidebar uses, so React Query
// dedupes it rather than issuing a second request.
export default function GroupStats({ groupId, memberCount }) {
  const { data: groups = [], status } = useQuery({
    queryKey: ['groups'],
    queryFn: listGroups,
  });

  const summary = groups.find((group) => group.id === groupId);
  const openTickets = status === 'success' && summary ? summary.open_ticket_count : '—';

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <Stat icon={User} label="Members" value={memberCount} />
      <Stat icon={Ticket} label="Open Issues" value={openTickets} />
      {/* No activity tracking in the schema yet — real once it exists. */}
      <Stat icon={Clock} label="Last Activity" value="—" />
    </div>
  );
}
