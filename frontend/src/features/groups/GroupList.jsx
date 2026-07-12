import { Link } from 'react-router-dom';
import GroupIcon from '../../components/ui/icons/GroupIcon';
import RoleIcon from '../../components/ui/icons/RoleIcon';
import MembersIcon from '../../components/ui/icons/MembersIcon';
import TicketsIcon from '../../components/ui/icons/TicketsIcon';
import ActivityIcon from '../../components/ui/icons/ActivityIcon';
import { isGroupAdmin } from '../../utils/roles';

export default function GroupList({ groups }) {
  if (groups.length === 0) {
    return <p className="text-sm text-slate-400">You're not in any groups yet.</p>;
  }

  return (
    <div className="overflow-x-auto rounded-lg border border-white/10">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Group
                <GroupIcon className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Role
                <RoleIcon className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Members
                <MembersIcon className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Open Tickets
                <TicketsIcon className="h-4 w-4 text-slate-400" />
              </span>
            </th>
            <th className="px-4 py-3">
              <span className="flex items-center gap-2">
                Last Activity
                <ActivityIcon className="h-4 w-4 text-slate-400" />
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
                {isGroupAdmin(group.role) ? 'Group Admin' : 'Contributor'}
              </td>
              <td className="px-4 py-3 text-slate-300">{group.member_count}</td>
              {/* No ticket module yet, and no activity-tracking in the schema — real
                  once both exist. */}
              <td className="px-4 py-3 text-slate-500">—</td>
              <td className="px-4 py-3 text-slate-500">—</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
