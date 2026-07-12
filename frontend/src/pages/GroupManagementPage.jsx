import { useParams } from 'react-router-dom';
import { useGroup } from '../hooks/useGroup';
import { useAuth } from '../hooks/useAuth';
import { isGroupAdmin } from '../utils/roles';

export default function GroupManagementPage() {
  const { id } = useParams();
  const { user } = useAuth();
  const { group, members, status } = useGroup(id);

  if (status === 'loading') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-slate-400">Loading…</p>
      </section>
    );
  }

  if (status === 'error') {
    return (
      <section className="mx-auto max-w-2xl px-4 py-20 sm:px-6 lg:px-8">
        <p className="text-sm text-red-500">
          Couldn't load this group. You may not be a member, or it may not exist.
        </p>
      </section>
    );
  }

  const myRole = members.find((member) => member.user_id === user.id)?.role;

  return (
    <section className="mx-auto flex max-w-2xl flex-col gap-8 px-4 py-20 sm:px-6 lg:px-8">
      <div>
        <h1 className="text-2xl font-bold text-white">{group.name}</h1>
        {myRole && (
          <p className="text-sm text-slate-400">
            Your role: {isGroupAdmin(myRole) ? 'Group Admin' : 'Contributor'}
          </p>
        )}
      </div>

      <div className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold text-white">Members</h2>
        <ul className="flex flex-col gap-2">
          {members.map((member) => (
            <li
              key={member.id}
              className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 py-3"
            >
              <div>
                <p className="text-sm font-medium text-white">{member.name}</p>
                <p className="text-xs text-slate-400">{member.email}</p>
              </div>
              <span className="rounded-full bg-white/10 px-3 py-1 text-xs font-medium text-slate-300">
                {isGroupAdmin(member.role) ? 'Group Admin' : 'Contributor'}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
