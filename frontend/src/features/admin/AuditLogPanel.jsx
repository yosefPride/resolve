import { useEffect, useState } from 'react';
import { formatDateTime } from '../../utils/format';
import { listAuditLog, listUsers, listGroups } from '../../services/admin.service';

const ACTION_LABELS = {
  succession: 'Succession',
  group_auto_deleted: 'Group auto-deleted',
};

function shortId(id) {
  return id ? `${id.slice(0, 8)}…` : '—';
}

function toMap(items) {
  return Object.fromEntries(items.map((item) => [item.id, item]));
}

// Best-effort name for an id. Names resolve for entities that still exist
// (successor, performing admin, and the group in a succession entry). The
// *deleted* user is always gone, and an auto-deleted group is gone too, so
// those fall back to a shortened id (full id available on hover via title).
function displayName(id, map) {
  return map[id]?.name || shortId(id);
}

export default function AuditLogPanel() {
  const [entries, setEntries] = useState([]);
  const [status, setStatus] = useState('loading'); // loading | ready | error
  const [usersById, setUsersById] = useState({});
  const [groups, setGroups] = useState([]);
  const [groupsById, setGroupsById] = useState({});
  const [userOptions, setUserOptions] = useState([]); // distinct deleted_user_ids
  const [groupFilter, setGroupFilter] = useState('');
  const [userFilter, setUserFilter] = useState('');

  useEffect(() => {
    let cancelled = false;
    // Load the full log plus the user/group lists once: the lists resolve ids to
    // names and populate the filter dropdowns. Deleted-user options come from the
    // log itself (deleted users aren't in the user list), computed here so they
    // stay stable as filters narrow the displayed rows.
    Promise.all([listAuditLog(), listUsers(), listGroups()])
      .then(([logEntries, users, groupList]) => {
        if (cancelled) return;
        setEntries(logEntries);
        setUsersById(toMap(users));
        setGroups(groupList);
        setGroupsById(toMap(groupList));
        setUserOptions([...new Set(logEntries.map((entry) => entry.deleted_user_id))]);
        setStatus('ready');
      })
      .catch(() => {
        if (cancelled) return;
        setStatus('error');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Event-driven refetch (not an effect) so the two independent filters can
  // combine and the backend's ?group_id=/?user_id= params do the narrowing.
  function applyFilters(nextGroup, nextUser) {
    setGroupFilter(nextGroup);
    setUserFilter(nextUser);
    listAuditLog({ groupId: nextGroup || undefined, userId: nextUser || undefined })
      .then(setEntries)
      .catch(() => setStatus('error'));
  }

  if (status === 'loading') return <p className="text-sm text-slate-400">Loading…</p>;
  if (status === 'error') return <p className="text-sm text-red-500">Failed to load the audit log.</p>;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-3">
        <label className="flex flex-col gap-1 text-xs text-slate-400">
          Group
          <select
            value={groupFilter}
            onChange={(event) => applyFilters(event.target.value, userFilter)}
            className="rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
          >
            <option value="">All groups</option>
            {groups.map((group) => (
              <option key={group.id} value={group.id}>
                {group.name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-xs text-slate-400">
          Deleted user
          <select
            value={userFilter}
            onChange={(event) => applyFilters(groupFilter, event.target.value)}
            className="rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
          >
            <option value="">All users</option>
            {userOptions.map((id) => (
              <option key={id} value={id}>
                {displayName(id, usersById)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {entries.length === 0 ? (
        <p className="text-sm text-slate-400">No audit entries.</p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-white/10">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
                <th className="px-4 py-3">Action</th>
                <th className="px-4 py-3">Group</th>
                <th className="px-4 py-3">Deleted user</th>
                <th className="px-4 py-3">Successor</th>
                <th className="px-4 py-3">Performed by</th>
                <th className="px-4 py-3">When</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry) => (
                <tr key={entry.id} className="border-b border-white/5 last:border-0 hover:bg-white/5">
                  <td className="px-4 py-3">
                    <span className="rounded-full bg-white/10 px-2.5 py-1 text-xs font-medium text-slate-200">
                      {ACTION_LABELS[entry.action] || entry.action}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-slate-300" title={entry.group_id}>
                    {displayName(entry.group_id, groupsById)}
                  </td>
                  <td className="px-4 py-3 text-slate-300" title={entry.deleted_user_id}>
                    {displayName(entry.deleted_user_id, usersById)}
                  </td>
                  <td className="px-4 py-3 text-slate-300" title={entry.successor_user_id || ''}>
                    {entry.successor_user_id ? displayName(entry.successor_user_id, usersById) : '—'}
                  </td>
                  <td className="px-4 py-3 text-slate-300" title={entry.performed_by}>
                    {displayName(entry.performed_by, usersById)}
                  </td>
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
