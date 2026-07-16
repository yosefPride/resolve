import { useEffect, useState } from 'react';
import UserTable from '../users/UserTable';
import { listUsers } from '../../services/admin.service';

export default function UsersPanel() {
  const [users, setUsers] = useState([]);
  const [status, setStatus] = useState('loading');

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

  if (status === 'loading') return <p className="text-sm text-slate-400">Loading…</p>;
  if (status === 'error') return <p className="text-sm text-red-500">Failed to load users.</p>;
  return <UserTable users={users} />;
}
