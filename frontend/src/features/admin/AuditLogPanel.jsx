import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { formatDateTime } from '../../utils/format';
import { listAuditLog } from '../../services/admin.service';
import Badge from '../../components/ui/Badge';

const ACTION_LABELS = {
  succession: 'Succession',
  group_auto_deleted: 'Team auto-deleted',
};

// Distinct { id, name } pairs from the log, first-seen order. The log carries
// snapshotted names, so this covers entities that no longer exist (deleted
// users, auto-deleted groups) — which a lookup against /admin/users or
// /admin/groups could not.
function distinctBy(entries, idKey, nameKey) {
  const seen = new Map();
  for (const entry of entries) {
    if (!seen.has(entry[idKey])) seen.set(entry[idKey], entry[nameKey]);
  }
  return [...seen.entries()].map(([id, name]) => ({ id, name }));
}

export default function AuditLogPanel() {
  const [groupFilter, setGroupFilter] = useState('');
  const [userFilter, setUserFilter] = useState('');

  const { data: entries = [], status } = useQuery({
    queryKey: ['admin', 'auditLog'],
    queryFn: listAuditLog,
  });

  // Options come from the full loaded log so they stay stable while filtering.
  const groupOptions = useMemo(() => distinctBy(entries, 'group_id', 'group_name'), [entries]);
  const userOptions = useMemo(
    () => distinctBy(entries, 'deleted_user_id', 'deleted_user_name'),
    [entries],
  );

  // The log is low-volume system metadata, so filter in memory rather than
  // round-tripping the backend's ?group_id=/?user_id= params per change.
  const visible = entries.filter(
    (entry) =>
      (!groupFilter || entry.group_id === groupFilter) &&
      (!userFilter || entry.deleted_user_id === userFilter),
  );

  if (status === 'pending') return <p className="text-sm text-slate-400">Loading…</p>;
  if (status === 'error') return <p className="text-sm text-red-500">Failed to load the audit log.</p>;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-3">
        <label className="flex flex-col gap-1 text-xs text-slate-400">
          Team
          <select
            value={groupFilter}
            onChange={(event) => setGroupFilter(event.target.value)}
            className="rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
          >
            <option value="">All teams</option>
            {groupOptions.map((option) => (
              <option key={option.id} value={option.id}>
                {option.name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-xs text-slate-400">
          Deleted user
          <select
            value={userFilter}
            onChange={(event) => setUserFilter(event.target.value)}
            className="rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
          >
            <option value="">All users</option>
            {userOptions.map((option) => (
              <option key={option.id} value={option.id}>
                {option.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      {visible.length === 0 ? (
        <p className="text-sm text-slate-400">No audit entries.</p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-white/10">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
                <th className="px-4 py-3">Action</th>
                <th className="px-4 py-3">Team</th>
                <th className="px-4 py-3">Deleted user</th>
                <th className="px-4 py-3">Successor</th>
                <th className="px-4 py-3">Performed by</th>
                <th className="px-4 py-3">When</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((entry) => (
                <tr key={entry.id} className="border-b border-white/5 last:border-0 hover:bg-white/5">
                  <td className="px-4 py-3">
                    <Badge>{ACTION_LABELS[entry.action] || entry.action}</Badge>
                  </td>
                  <td className="px-4 py-3 text-slate-300">{entry.group_name}</td>
                  <td className="px-4 py-3 text-slate-300">{entry.deleted_user_name}</td>
                  <td className="px-4 py-3 text-slate-300">{entry.successor_user_name || '—'}</td>
                  <td className="px-4 py-3 text-slate-300">{entry.performed_by_name}</td>
                  <td className="px-4 py-3 text-slate-400">{formatDateTime(entry.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
