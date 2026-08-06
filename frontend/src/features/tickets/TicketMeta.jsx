import { Link } from 'react-router-dom';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Check, Users } from 'lucide-react';
import { useUpdateTicket } from '../../hooks/useTickets';
import { errorMessage } from '../../utils/errors';
import Avatar from '../../components/ui/Avatar';
import Badge from '../../components/ui/Badge';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from './badgeVariants';

const STATUS_OPTIONS = ['open', 'closed'];
const PRIORITY_OPTIONS = ['low', 'high', 'critical'];

// The row's badge as a value picker: the badge itself is the trigger, the
// menu lists the enum values with a check on the current one. Selecting the
// current value is a no-op rather than a wasted PATCH.
function BadgeDropdown({ ariaLabel, value, variant, options, disabled, onSelect }) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        disabled={disabled}
        aria-label={ariaLabel}
        className="rounded-full outline-none focus-visible:ring-1 focus-visible:ring-white disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Badge size="sm" variant={variant}>
          {capitalize(value)}
        </Badge>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          className="z-50 w-36 rounded-lg border border-white/10 bg-neutral-800 py-1 shadow-2xl shadow-black/50"
        >
          {options.map((option) => (
            <DropdownMenu.Item
              key={option}
              onSelect={() => option !== value && onSelect(option)}
              className="flex cursor-pointer items-center justify-between px-4 py-2 text-sm text-slate-300 outline-none transition-colors data-highlighted:bg-white/10"
            >
              {capitalize(option)}
              {option === value && <Check className="h-4 w-4 text-sky-400" />}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

// Right-rail summary card: Priority, Status (admin-editable inline), Reporter,
// Team. isAdmin gates the dropdowns — RBAC is UX-only here, the backend
// rejects a non-Group-Admin PATCH regardless (docs/rbac.md).
export default function TicketMeta({ ticket, teamName, groupId, isAdmin }) {
  const updateMutation = useUpdateTicket(groupId, ticket.id);

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-6">
      <dl className="flex flex-col gap-4">
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Priority</dt>
          <dd>
            {isAdmin ? (
              <BadgeDropdown
                ariaLabel="Change priority"
                value={ticket.priority}
                variant={PRIORITY_VARIANT[ticket.priority]}
                options={PRIORITY_OPTIONS}
                disabled={updateMutation.isPending}
                onSelect={(priority) => updateMutation.mutate({ priority })}
              />
            ) : (
              <Badge size="sm" variant={PRIORITY_VARIANT[ticket.priority]}>
                {capitalize(ticket.priority)}
              </Badge>
            )}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Status</dt>
          <dd>
            {isAdmin ? (
              <BadgeDropdown
                ariaLabel="Change status"
                value={ticket.status}
                variant={STATUS_VARIANT[ticket.status]}
                options={STATUS_OPTIONS}
                disabled={updateMutation.isPending}
                onSelect={(status) => updateMutation.mutate({ status })}
              />
            ) : (
              <Badge size="sm" variant={STATUS_VARIANT[ticket.status]}>
                {capitalize(ticket.status)}
              </Badge>
            )}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="shrink-0 text-sm text-slate-500">Reporter</dt>
          <dd className="flex min-w-0 items-center gap-2">
            <Avatar name={ticket.created_by_name} seed={ticket.created_by} size="sm" />
            <span className="truncate text-sm text-slate-200">{ticket.created_by_name}</span>
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="shrink-0 text-sm text-slate-500">Team</dt>
          <dd className="flex min-w-0 items-center gap-2 text-sm text-slate-200">
            <Users className="h-4 w-4 shrink-0 text-slate-400" />
            {teamName ? (
              <Link to={`/groups/${groupId}`} className="truncate hover:text-white hover:underline">
                {teamName}
              </Link>
            ) : (
              '—'
            )}
          </dd>
        </div>
      </dl>

      {updateMutation.isError && (
        <p className="mt-4 text-sm text-red-500">
          {errorMessage(updateMutation.error, 'Failed to update issue.')}
        </p>
      )}
    </div>
  );
}
