import { useEffect, useState } from 'react';
import GroupList from '../features/groups/GroupList';
import CreateGroupForm from '../features/groups/CreateGroupForm';
import Modal from '../components/ui/Modal';
import { listGroups } from '../services/groups.service';
import { GROUP_ROLES } from '../utils/roles';

export default function MyGroupsPage() {
  const [groups, setGroups] = useState([]);
  const [status, setStatus] = useState('loading');
  const [isCreating, setIsCreating] = useState(false);

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
    // The creator is always Group Admin and the sole member at creation time
    // (see GroupService::create_group) — safe to fill these in client-side
    // rather than round-tripping to GET /groups just to get the same values.
    setGroups((prev) => [...prev, { ...group, role: GROUP_ROLES.GROUP_ADMIN, member_count: 1 }]);
    setIsCreating(false);
  }

  return (
    <section className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Your teams</h1>
        <button
          type="button"
          onClick={() => setIsCreating(true)}
          className="flex items-center gap-1.5 rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-black hover:ring-1 hover:ring-white  hover:text-white disabled:cursor-not-allowed disabled:bg-white/50 disabled:text-black/50"
        >
          <span className="text-base leading-none">+</span>
          Create team
        </button>
      </div>

      <Modal isOpen={isCreating} onClose={() => setIsCreating(false)} title="Create a team">
        <CreateGroupForm onCreated={handleCreated} />
      </Modal>

      {status === 'loading' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load teams.</p>}
      {status === 'ready' && <GroupList groups={groups} />}
    </section>
  );
}
