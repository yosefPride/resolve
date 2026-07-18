import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import GroupList from '../features/groups/GroupList';
import CreateGroupForm from '../features/groups/CreateGroupForm';
import Modal from '../components/ui/Modal';
import { listGroups } from '../services/groups.service';
import Button from '../components/ui/Button';

export default function MyGroupsPage() {
  const queryClient = useQueryClient();
  const [isCreating, setIsCreating] = useState(false);

  const { data: groups = [], status } = useQuery({
    queryKey: ['groups'],
    queryFn: listGroups,
  });

  function handleCreated() {
    // The new group is server truth now, so just invalidate the cache — this
    // list (and any future consumer of ['groups'], e.g. the sidebar) refetches
    // automatically. No more hand-fabricated optimistic group object.
    queryClient.invalidateQueries({ queryKey: ['groups'] });
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

      {status === 'pending' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load teams.</p>}
      {status === 'success' && <GroupList groups={groups} />}
    </section>
  );
}
