import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Link2, X } from 'lucide-react';
import { useAuth } from '../../hooks/useAuth';
import { useCreateLink, useDeleteLink, useLinks } from '../../hooks/useLinks';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import { useSubmit } from '../../hooks/useSubmit';
import { listTickets } from '../../services/tickets.service';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../tickets/badgeVariants';
import CollectionSection from './CollectionSection';

// blocks/is_blocked_by draw the eye (red) since blocked work is the case that
// most needs attention; duplicates/is_duplicated_by are muted (informational,
// nothing actionable); relates_to is the plain default.
const LINK_LABEL_VARIANT = {
  blocks: 'danger',
  is_blocked_by: 'danger',
  relates_to: 'neutral',
  duplicates: 'outline',
  is_duplicated_by: 'outline',
};

const LINK_LABEL_TEXT = {
  blocks: 'Blocks',
  is_blocked_by: 'Is blocked by',
  relates_to: 'Relates to',
  duplicates: 'Duplicates',
  is_duplicated_by: 'Is duplicated by',
};

function LinkRow({ link, groupId }) {
  return (
    <div className="flex min-w-0 items-center gap-3">
      <Badge size="sm" variant={LINK_LABEL_VARIANT[link.label]}>
        {LINK_LABEL_TEXT[link.label]}
      </Badge>
      <Link
        to={`/tickets/${link.other_ticket_id}?group=${groupId}`}
        className="flex min-w-0 items-center gap-2 hover:underline"
      >
        <span className="shrink-0 text-xs font-medium text-slate-500">
          #{link.other_ticket_number}
        </span>
        <span className="truncate text-sm text-white">{link.other_ticket_title}</span>
      </Link>
      <Badge size="sm" variant={STATUS_VARIANT[link.other_ticket_status]}>
        {capitalize(link.other_ticket_status)}
      </Badge>
      <Badge size="sm" variant={PRIORITY_VARIANT[link.other_ticket_priority]}>
        {capitalize(link.other_ticket_priority)}
      </Badge>
    </div>
  );
}

// Search-as-you-type list of candidate tickets to link against, same
// debounced pattern as the admin panels (UsersPanel/GroupsPanel) — raw
// useQuery here rather than useTicketList, so it can stay disabled until
// there's an actual search term (an empty term would otherwise fetch the
// group's whole first page of tickets as "suggestions", which isn't useful).
function TicketPicker({ groupId, currentTicketId, onSelect }) {
  const [query, setQuery] = useState('');
  const debouncedQuery = useDebouncedValue(query, 300);
  const term = debouncedQuery.trim();

  const { data } = useQuery({
    queryKey: ['tickets', groupId, { q: term, perPage: 5 }],
    queryFn: () => listTickets(groupId, { q: term, perPage: 5 }),
    enabled: Boolean(groupId) && term.length > 0,
  });

  // The ticket itself can't be a candidate — linking to self is rejected
  // server-side anyway, but filtering it out here avoids a wasted round trip.
  const results = (data?.items ?? []).filter((ticket) => ticket.id !== currentTicketId);

  return (
    <div className="flex flex-col gap-2">
      <Input
        type="search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Search issues by title…"
        aria-label="Search issues to link"
      />
      {term.length > 0 && results.length > 0 && (
        <ul className="flex flex-col gap-1 rounded-lg border border-white/10 bg-neutral-950 p-1">
          {results.map((ticket) => (
            <li key={ticket.id}>
              <button
                type="button"
                onClick={() => onSelect(ticket)}
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-slate-300 hover:bg-white/10"
              >
                <span className="shrink-0 text-xs text-slate-500">#{ticket.ticket_number}</span>
                <span className="truncate">{ticket.title}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {term.length > 0 && results.length === 0 && (
        <p className="text-xs text-slate-500">No matching issues.</p>
      )}
    </div>
  );
}

// Rendered inside a Modal (see CollectionSection) — that caps the width, so
// neither this form nor TicketPicker need their own width constraints.
function AddLinkForm({ groupId, ticketId, onDone, onCancel }) {
  const [selectedTicket, setSelectedTicket] = useState(null);
  const [relationType, setRelationType] = useState('relates_to');
  const createLink = useCreateLink(groupId, ticketId);
  const { error, submit } = useSubmit(async () => {
    await createLink.mutateAsync({ targetTicketId: selectedTicket.id, relationType });
    onDone();
  }, 'Failed to add link.');

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <span className="text-sm text-slate-300">Issue</span>
        {selectedTicket ? (
          <div className="flex items-center justify-between gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2">
            <span className="min-w-0 truncate text-sm text-white">
              #{selectedTicket.ticket_number} {selectedTicket.title}
            </span>
            <button
              type="button"
              onClick={() => setSelectedTicket(null)}
              aria-label="Change issue"
              className="shrink-0 text-slate-400 hover:text-white"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        ) : (
          <TicketPicker groupId={groupId} currentTicketId={ticketId} onSelect={setSelectedTicket} />
        )}
      </div>

      <Field label="Relation">
        <Select value={relationType} onChange={(event) => setRelationType(event.target.value)}>
          <option value="blocks">Blocks</option>
          <option value="relates_to">Relates to</option>
          <option value="duplicates">Duplicates</option>
        </Select>
      </Field>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <div className="mt-2 flex justify-end gap-3">
        <Button type="button" variant="ghost" disabled={createLink.isPending} onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={!selectedTicket || createLink.isPending}>
          {createLink.isPending ? 'Adding…' : 'Add link'}
        </Button>
      </div>
    </form>
  );
}

export default function LinksSection({ groupId, ticketId, isAdmin }) {
  const { user } = useAuth();
  const query = useLinks(groupId, ticketId);
  const deleteLink = useDeleteLink(groupId, ticketId);

  return (
    <CollectionSection
      icon={Link2}
      title="Linked issues"
      addLabel="Add link"
      addTitle="Add link"
      loadingText="Loading links…"
      emptyText="No linked issues yet."
      loadErrorFallback="Couldn't load links."
      deleteErrorFallback="Failed to remove link."
      query={query}
      deleteMutation={deleteLink}
      canDelete={(link) => link.created_by === user.id || isAdmin}
      renderRow={(link) => <LinkRow link={link} groupId={groupId} />}
      renderAddForm={(close) => (
        <AddLinkForm groupId={groupId} ticketId={ticketId} onDone={close} onCancel={close} />
      )}
    />
  );
}
