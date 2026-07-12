import { useEffect, useState } from 'react';
import GroupList from '../features/groups/GroupList';
import CreateGroupForm from '../features/groups/CreateGroupForm';
import { listGroups } from '../services/groups.service';

export default function GroupSelectionPage() {
  const [groups, setGroups] = useState([]);
  const [status, setStatus] = useState('loading');

  useEffect(() => {
    let cancelled = false;
    listGroups()
      .then((data) => {
        if (cancelled) return;
        setGroups(data);
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

  function handleCreated(group) {
    setGroups((prev) => [...prev, group]);
  }

  return (
    <section className="mx-auto flex max-w-md flex-col gap-8 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Your groups</h1>

      {status === 'loading' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load groups.</p>}
      {status === 'ready' && <GroupList groups={groups} />}

      <div className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold text-white">Create a group</h2>
        <CreateGroupForm onCreated={handleCreated} />
      </div>
    </section>
  );
}
