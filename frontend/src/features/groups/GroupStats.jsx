import { useQuery } from '@tanstack/react-query';
import { Clock, Ticket, User } from 'lucide-react';
import { listGroups } from '../../services/groups.service';
import StatTile from '../../components/ui/StatTile';

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
    <div className="flex flex-row gap-3">
      <div className="flex-1">
        <StatTile icon={User} label="Members" value={memberCount} />
      </div>
      <div className="flex-1">
        <StatTile icon={Ticket} label="Open Issues" value={openTickets} />
      </div>
      {/* No activity tracking in the schema yet — real once it exists. */}
      <div className="flex-1">
        <StatTile icon={Clock} label="Last Activity" value="—" />
      </div>
    </div>
  );
}
