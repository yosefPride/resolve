import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Ticket, User as UserIcon, Users } from 'lucide-react';
import { listGroups } from '../../services/groups.service';
import { useDashboardOverview } from '../../hooks/useDashboardOverview';
import { useAuth } from '../../hooks/useAuth';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
import CreateGroupForm from '../groups/CreateGroupForm';

// Tailwind's class scanner needs full literal class names, so these can't be
// built with a template string off the `color` key below.
const STAT_COLORS = {
  sky: { value: 'text-sky-300', chip: 'bg-sky-500/15 text-sky-300' },
  amber: { value: 'text-amber-300', chip: 'bg-amber-500/15 text-amber-300' },
  rose: { value: 'text-rose-300', chip: 'bg-rose-500/15 text-rose-300' },
  violet: { value: 'text-violet-300', chip: 'bg-violet-500/15 text-violet-300' },
};

// Shares the ['groups'] query key with Sidebar and GroupStats, so this page
// is usually a cache hit rather than a fresh request, and stays in sync with
// any create/rename/delete those pages invalidate. No dedicated dashboard
// endpoint exists or is needed: GET /groups already carries every field used
// for the Teams/Open Issues tiles and the team cards (member_count,
// open_ticket_count, role) per team; the priority/mine tiles come from
// useDashboardOverview instead (see that hook for why).
export default function DashboardStats() {
  const [isCreating, setIsCreating] = useState(false);
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });
  const { tickets } = useDashboardOverview(groups);

  function handleCreated() {
    queryClient.invalidateQueries({ queryKey: ['groups'] });
    setIsCreating(false);
  }

  if (status === 'pending') {
    return <p className="text-sm text-slate-400">Loading your teams…</p>;
  }

  if (status === 'error') {
    return <p className="text-sm text-red-500">Couldn't load your teams.</p>;
  }

  if (groups.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 border-t border-white/10 py-10 text-center">
        <p className="text-sm text-slate-400">You're not part of any team yet.</p>
        <Button onClick={() => setIsCreating(true)}>Create a team</Button>
        <Modal isOpen={isCreating} onClose={() => setIsCreating(false)} title="Create a team">
          <CreateGroupForm onCreated={handleCreated} />
        </Modal>
      </div>
    );
  }

  const totalOpenIssues = groups.reduce((sum, group) => sum + group.open_ticket_count, 0);
  const criticalOrHighOpen = tickets.filter(
    (ticket) => ticket.status === 'open' && (ticket.priority === 'critical' || ticket.priority === 'high'),
  ).length;
  const myOpenTickets = tickets.filter(
    (ticket) => ticket.status === 'open' && ticket.created_by === user?.id,
  ).length;

  return (
    <div className="grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-4">
      {[
        { icon: Users, label: 'Teams', value: groups.length, color: 'sky' },
        { icon: Ticket, label: 'Open Issues', value: totalOpenIssues, color: 'amber' },
        { icon: AlertTriangle, label: 'Critical/High Open', value: criticalOrHighOpen, color: 'rose' },
        { icon: UserIcon, label: 'My Open Issues', value: myOpenTickets, color: 'violet' },
      ].map(({ icon: Icon, label, value, color }) => (
        <div
          key={label}
          className="flex flex-col gap-2 rounded-xl border border-white/10 bg-white/5 p-3 sm:gap-4 sm:p-5"
        >
          <span className="flex items-center gap-1.5 text-xs font-semibold text-slate-200 sm:gap-2 sm:text-base">
            <span
              className={`flex h-7 w-7 items-center justify-center rounded-full sm:h-10 sm:w-10 ${STAT_COLORS[color].chip}`}
            >
              <Icon className="h-3.5 w-3.5 sm:h-5 sm:w-5" />
            </span>
            {label}
          </span>
          <span className={`text-2xl font-bold sm:text-4xl ${STAT_COLORS[color].value}`}>{value}</span>
        </div>
      ))}
    </div>
  );
}
