import { useEffect, useState } from 'react';
import UserTable from '../users/UserTable';
import DeleteUserModal from './DeleteUserModal';
import { listUsers } from '../../services/admin.service';
import { useAuth } from '../../hooks/useAuth';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';

export default function UsersPanel() {
  const { user: currentUser } = useAuth();
  const [users, setUsers] = useState([]);
  const [status, setStatus] = useState('loading');
  const [deleteTarget, setDeleteTarget] = useState(null);
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, 300);

  // Refetches whenever the debounced term changes. Status only flips back to
  // 'loading' for the initial load, so typing swaps results in place rather
  // than flashing a spinner (and the input keeps focus). The cancelled flag
  // drops a stale response if the term changed again before it resolved.
  useEffect(() => {
    let cancelled = false;
    listUsers(debouncedSearch)
      .then((data) => {
        if (cancelled) return;
        setUsers(data);
        setStatus('ready');
      })
      .catch(() => {
        if (cancelled) return;
        setStatus('error');
      });
    return () => {
      cancelled = true;
    };
  }, [debouncedSearch]);

  async function handleDeleted() {
    setDeleteTarget(null);
    // Refetch server truth (a deletion may have cascaded auto-deleted groups),
    // keeping the active search filter applied.
    const data = await listUsers(debouncedSearch);
    setUsers(data);
  }

  return (
    <>
      <input
        type="search"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder="Search by name or email"
        aria-label="Search users"
        className="mb-4 w-full max-w-sm rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
      />

      {status === 'loading' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load users.</p>}
      {status === 'ready' &&
        (users.length === 0 ? (
          <p className="text-sm text-slate-400">
            {debouncedSearch ? `No users match “${debouncedSearch}”.` : 'No users found.'}
          </p>
        ) : (
          <UserTable users={users} currentUserId={currentUser?.id} onDelete={setDeleteTarget} />
        ))}

      {deleteTarget && (
        <DeleteUserModal
          user={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={handleDeleted}
        />
      )}
    </>
  );
}
