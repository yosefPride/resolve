import { useEffect, useState } from 'react';
import GroupList from '../features/groups/GroupList';
import CreateGroupForm from '../features/groups/CreateGroupForm';
import Modal from '../components/ui/Modal';
import { listGroups } from '../services/groups.service';
import { GROUP_ROLES } from '../utils/roles';
import Button from '../components/ui/Button';

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
    setGroups((prev) => [
      ...prev,
      { ...group, role: GROUP_ROLES.GROUP_ADMIN, member_count: 1, open_ticket_count: 0 },
    ]);
    setIsCreating(false);
  }

  return (
    <section className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-white">Your teams</h1>
        <Button onClick={() => setIsCreating(true)} className="gap-1.5">
          <span className="text-base leading-none">+</span>
          Create team
        </Button>
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
