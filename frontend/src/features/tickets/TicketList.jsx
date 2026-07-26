import { useState } from 'react';
import { useTicketList } from '../../hooks/useTickets';
import { useGroup } from '../../hooks/useGroup';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import TicketFilters from './TicketFilters';
import TicketCard from './TicketCard';
import Button from '../../components/ui/Button';

const PER_PAGE = 20;

export default function TicketList({ groupId }) {
  const [search, setSearch] = useState('');
  const [status, setStatus] = useState('');
  const [priority, setPriority] = useState('');
  const [creator, setCreator] = useState('');
  const [page, setPage] = useState(1);
  const debouncedSearch = useDebouncedValue(search, 300);
  const { members } = useGroup(groupId);

  const filters = {
    q: debouncedSearch,
    status,
    priority,
    creator,
    page,
    perPage: PER_PAGE,
  };

  const { data, status: queryStatus } = useTicketList(groupId, filters);
  const tickets = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PER_PAGE));

  // Any filter change resets to page 1 — otherwise a narrower result set can
  // leave the view stranded on a page number past the new total.
  function updateFilter(setter) {
    return (value) => {
      setter(value);
      setPage(1);
    };
  }

  return (
    <div className="flex flex-col gap-4">
      <TicketFilters
        search={search}
        onSearchChange={updateFilter(setSearch)}
        status={status}
        onStatusChange={updateFilter(setStatus)}
        priority={priority}
        onPriorityChange={updateFilter(setPriority)}
        creator={creator}
        onCreatorChange={updateFilter(setCreator)}
        members={members}
      />

      {queryStatus === 'pending' && <p className="text-sm text-slate-400">Loading…</p>}
      {queryStatus === 'error' && <p className="text-sm text-red-500">Failed to load issues.</p>}
      {queryStatus === 'success' && tickets.length === 0 && (
        <p className="text-sm text-slate-400">No issues match these filters.</p>
      )}

      {tickets.length > 0 && (
        <div className="flex flex-col gap-2">
          {tickets.map((ticket) => (
            <TicketCard key={ticket.id} ticket={ticket} groupId={groupId} />
          ))}
        </div>
      )}

      {totalPages > 1 && (
        <div className="flex items-center justify-between text-sm text-slate-400">
          <span>
            Page {page} of {totalPages}
          </span>
          <div className="flex gap-2">
            <Button
              variant="ghost"
              size="sm"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={page >= totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
