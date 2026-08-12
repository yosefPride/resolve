import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';
import { ChevronDown, Plus } from 'lucide-react';
import { listGroups } from '../services/groups.service';
import TicketList from '../features/tickets/TicketList';
import CreateTicketForm from '../features/tickets/CreateTicketForm';
import Button from '../components/ui/Button';
import DropdownMenu, { DropdownMenuItem } from '../components/ui/DropdownMenu';
import Modal from '../components/ui/Modal';

// The selected team lives in the URL (?group=<id>), not component state, so a
// refresh or a shared link keeps pointing at the same team's issues — there is
// still no "active group" held anywhere globally (docs/frontend.md).
export default function TicketsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const groupId = searchParams.get('group') || '';
  const [isCreating, setIsCreating] = useState(false);

  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

  // No team chosen yet (fresh visit, no ?group in the URL): default to the
  // first team the user belongs to, once the list has loaded.
  useEffect(() => {
    if (!groupId && groups.length > 0) {
      setSearchParams({ group: groups[0].id }, { replace: true });
    }
  }, [groupId, groups, setSearchParams]);

  const currentGroup = groups.find((group) => group.id === groupId);

  function selectGroup(id) {
    setSearchParams({ group: id });
  }

  if (status === 'pending') {
    return (
      <section>
        <p className="text-sm text-slate-400">Loading…</p>
      </section>
    );
  }

  if (status === 'error') {
    return (
      <section>
        <p className="text-sm text-red-500">Couldn't load your teams.</p>
      </section>
    );
  }

  if (groups.length === 0) {
    return (
      <section>
        <p className="text-sm text-slate-400">
          You're not in any team yet. Create one from the sidebar to start opening issues.
        </p>
      </section>
    );
  }

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <DropdownMenu
          align="start"
          width="w-56"
          trigger={
            <button
              type="button"
              className="group flex min-w-0 max-w-full items-center gap-2 text-2xl font-bold text-white outline-none"
            >
              <span className="truncate">{currentGroup?.name ?? 'Select a team'}</span>
              <ChevronDown className="h-5 w-5 shrink-0 transition-transform group-data-[state=open]:rotate-180" />
            </button>
          }
        >
          {groups.map((group) => (
            <DropdownMenuItem
              key={group.id}
              variant="plain"
              className={group.id === groupId ? 'text-white' : 'text-slate-300'}
              onSelect={() => selectGroup(group.id)}
            >
              {group.name}
            </DropdownMenuItem>
          ))}
        </DropdownMenu>

        <Button onClick={() => setIsCreating(true)} className="shrink-0">
          <Plus className="mr-1 h-4 w-4" />
          New issue
        </Button>
      </div>

      {/* key={groupId} forces a full remount on team switch — otherwise
          useTicketList's keepPreviousData briefly renders the previous team's
          tickets under the new team's heading while the new query loads. */}
      {groupId && <TicketList key={groupId} groupId={groupId} />}

      <Modal isOpen={isCreating} onClose={() => setIsCreating(false)} title="New issue">
        <CreateTicketForm groupId={groupId} onCreated={() => setIsCreating(false)} />
      </Modal>
    </section>
  );
}
