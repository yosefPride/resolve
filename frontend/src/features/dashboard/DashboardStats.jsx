import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { Ticket, Users } from 'lucide-react';
import { listGroups } from '../../services/groups.service';
import { isGroupAdmin } from '../../utils/roles';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
import StatTile from '../../components/ui/StatTile';
import CreateGroupForm from '../groups/CreateGroupForm';

// Shares the ['groups'] query key with Sidebar and GroupStats, so this page
// is usually a cache hit rather than a fresh request, and stays in sync with
// any create/rename/delete those pages invalidate. No dedicated dashboard
// endpoint exists or is needed: GET /groups already carries every field used
// here (member_count, open_ticket_count, role) per team.
export default function DashboardStats() {
  const [isCreating, setIsCreating] = useState(false);
  const queryClient = useQueryClient();
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

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

  return (
    <div className="flex flex-col gap-6 border-t border-white/10 pt-6">
      <div className="grid gap-3 sm:grid-cols-2">
        <StatTile icon={Users} label="Teams" value={groups.length} />
        <StatTile icon={Ticket} label="Open Issues" value={totalOpenIssues} />
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {groups.map((group) => (
          <Link
            key={group.id}
            to={`/tickets?group=${group.id}`}
            className="flex flex-col gap-2 rounded-lg border border-white/10 bg-white/5 p-4 text-left transition-colors hover:border-white/20 hover:bg-white/10"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="truncate font-semibold text-white">{group.name}</span>
              <Badge size="sm">{isGroupAdmin(group.role) ? 'Team Admin' : 'Contributor'}</Badge>
            </div>
            <p className="text-sm text-slate-400">
              {group.member_count} member{group.member_count === 1 ? '' : 's'} ·{' '}
              {group.open_ticket_count} open issue{group.open_ticket_count === 1 ? '' : 's'}
            </p>
          </Link>
        ))}
      </div>
    </div>
  );
}
