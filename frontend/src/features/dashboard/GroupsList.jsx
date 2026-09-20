import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { listGroups } from '../../services/groups.service';
import { isGroupAdmin } from '../../utils/roles';
import Badge from '../../components/ui/Badge';

// Shares the ['groups'] query key with DashboardStats/Sidebar/GroupStats, so
// this is normally a cache hit, not a fresh request. Renders null while
// DashboardStats is showing its own loading/error/onboarding state for that
// same query, so nothing duplicates or flashes empty above Recent activity.
export default function GroupsList() {
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

  if (status !== 'success' || groups.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-col gap-3">
      <h2 className="text-sm font-semibold text-white">Your teams</h2>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-1">
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
