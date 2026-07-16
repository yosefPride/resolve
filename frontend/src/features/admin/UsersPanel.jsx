import { useEffect, useState } from 'react';
import UserTable from '../users/UserTable';
import DeleteUserModal from './DeleteUserModal';
import { listUsers } from '../../services/admin.service';
import { useAuth } from '../../hooks/useAuth';

export default function UsersPanel() {
  const { user: currentUser } = useAuth();
  const [users, setUsers] = useState([]);
  const [status, setStatus] = useState('loading');
  const [deleteTarget, setDeleteTarget] = useState(null);

  useEffect(() => {
    let cancelled = false;
    listUsers()
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
  }, []);

  async function handleDeleted() {
    setDeleteTarget(null);
    // Refetch server truth (a deletion may have cascaded auto-deleted groups).
    const data = await listUsers();
    setUsers(data);
  }

  if (status === 'loading') return <p className="text-sm text-slate-400">Loading…</p>;
  if (status === 'error') return <p className="text-sm text-red-500">Failed to load users.</p>;

  return (
    <>
      <UserTable users={users} currentUserId={currentUser?.id} onDelete={setDeleteTarget} />
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
