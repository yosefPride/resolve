import { Link } from 'react-router-dom';
import { Users, Shield, User, Ticket, Clock } from 'lucide-react';
import { isGroupAdmin } from '../../utils/roles';

export default function GroupList({ groups }) {
  if (groups.length === 0) {
    return <p className="text-sm text-slate-400">You're not in any teams yet.</p>;
  }

  return (
    <div className="overflow-x-auto rounded-lg border border-white/10">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Team
                <Users className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Role
                <Shield className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Members
                <User className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Open Tickets
                <Ticket className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Last Activity
                <Clock className="h-4 w-4 text-slate-400" />
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          {groups.map((group) => (
            <tr key={group.id} className="border-b border-white/5 last:border-0 hover:bg-white/5">
              <td className="px-4 py-3">
                <Link to={`/groups/${group.id}`} className="font-medium text-white hover:text-blue-500 hover:underline">
                  {group.name}
                </Link>
              </td>
              <td className="px-4 py-3 text-slate-300">
                {isGroupAdmin(group.role) ? 'Team Admin' : 'Contributor'}
              </td>
              <td className="px-4 py-3 text-slate-300">{group.member_count}</td>
              <td className="px-4 py-3 text-slate-300">{group.open_ticket_count}</td>
              {/* No activity-tracking in the schema yet — real once it exists. */}
              <td className="px-4 py-3 text-slate-500">—</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
