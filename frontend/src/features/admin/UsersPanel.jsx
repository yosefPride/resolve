import { useState } from 'react';
import { useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import UserTable from '../users/UserTable';
import DeleteUserModal from './DeleteUserModal';
import { listUsers } from '../../services/admin.service';
import { useAuth } from '../../hooks/useAuth';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import Input from '../../components/ui/Input';

export default function UsersPanel() {
  const { user: currentUser } = useAuth();
  const queryClient = useQueryClient();
  const [deleteTarget, setDeleteTarget] = useState(null);
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, 300);

  // keepPreviousData keeps the current rows on screen while a new search term
  // fetches, so typing swaps results in place instead of flashing a spinner —
  // the behavior the old hand-rolled "don't reset status while typing" did.
  const { data: users = [], status } = useQuery({
    queryKey: ['admin', 'users', debouncedSearch],
    queryFn: () => listUsers(debouncedSearch),
    placeholderData: keepPreviousData,
  });

  function handleDeleted() {
    setDeleteTarget(null);
    // A deletion may cascade auto-deleted groups, so refresh both admin lists.
    queryClient.invalidateQueries({ queryKey: ['admin', 'users'] });
    queryClient.invalidateQueries({ queryKey: ['admin', 'groups'] });
  }

  return (
    <>
      <Input
        type="search"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder="Search by name or email"
        aria-label="Search users"
        className="mb-4 w-full max-w-sm text-sm"
      />

      {status === 'pending' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load users.</p>}
      {status === 'success' &&
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
