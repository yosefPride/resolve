import { useQuery } from '@tanstack/react-query';
import { Clock, Ticket, User } from 'lucide-react';
import { listGroups } from '../../services/groups.service';
import { formatRelativeTime } from '../../utils/format';
import StatTile from '../../components/ui/StatTile';

// Open issues and last activity both come from GET /groups, which reports
// them for every team the caller belongs to — and viewing a team requires
// membership, so this team is always in that list. Same ['groups'] key the
// sidebar uses, so React Query dedupes it rather than issuing a second
// request.
export default function GroupStats({ groupId, memberCount }) {
  const { data: groups = [], status } = useQuery({
    queryKey: ['groups'],
    queryFn: listGroups,
  });

  const summary = groups.find((group) => group.id === groupId);
  const openTickets = status === 'success' && summary ? summary.open_ticket_count : '—';
  // formatRelativeTime already falls back to '—' for null/undefined, which
  // covers both "still loading" and "brand-new group with no activity yet".
  const lastActivity = formatRelativeTime(summary?.last_activity_at);

  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
      <StatTile icon={User} label="Members" value={memberCount} />
      <StatTile icon={Ticket} label="Open Issues" value={openTickets} />
      <StatTile icon={Clock} label="Last Activity" value={lastActivity} />
    </div>
  );
}
