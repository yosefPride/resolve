import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  ArrowLeft,
  Check,
  ChevronDown,
  FileText,
  MessageSquare,
  MoreVertical,
  Pencil,
  Users,
} from 'lucide-react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useDeleteTicket, useUpdateTicket } from '../../hooks/useTickets';
import { useComments } from '../../hooks/useComments';
import { errorMessage } from '../../utils/errors';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import Avatar from '../../components/ui/Avatar';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
import CommentList from '../comments/CommentList';
import EditTicketForm from './EditTicketForm';
import TicketMeta from './TicketMeta';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from './badgeVariants';

const STATUS_OPTIONS = ['open', 'closed'];
const PRIORITY_OPTIONS = ['low', 'high', 'critical'];

// The field-row badge as a value picker: the badge itself is the trigger, the
// menu lists the enum values with a check on the current one. Selecting the
// current value is a no-op rather than a wasted PATCH.
function BadgeDropdown({ ariaLabel, value, variant, options, disabled, onSelect }) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        disabled={disabled}
        aria-label={ariaLabel}
        className="group inline-flex items-center gap-1 rounded-full outline-none focus-visible:ring-1 focus-visible:ring-white disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Badge variant={variant}>{capitalize(value)}</Badge>
        <ChevronDown className="h-3.5 w-3.5 text-slate-500 transition-colors group-hover:text-slate-300" />
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={6}
          className="z-50 w-36 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50"
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

function TabButton({ isActive, onClick, icon: Icon, children }) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={isActive}
      onClick={onClick}
      className={`-mb-px inline-flex items-center gap-2 border-b-2 px-1 pb-3 text-sm font-medium transition-colors ${
        isActive
          ? 'border-sky-400 text-white'
          : 'border-transparent text-slate-400 hover:text-slate-200'
      }`}
    >
      <Icon className="h-4 w-4" />
      {children}
    </button>
  );
}

// Owns the full detail layout: top bar (back link + admin actions), the
// main-column cards, and the right rail. isAdmin gates Edit/Delete/status —
// RBAC is UX-only here, the backend rejects a non-Group-Admin PATCH/DELETE
// regardless (docs/rbac.md).
export default function TicketDetail({ ticket, teamName, groupId, isAdmin }) {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState('details');
  const [isEditing, setIsEditing] = useState(false);
  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [deleteError, setDeleteError] = useState('');

  // Same query CommentList runs — React Query dedupes, so the tab count costs
  // no extra request. Tombstones stay in the thread but don't count.
  const { data: comments } = useComments(groupId, ticket.id);
  const commentCount = comments
    ? comments.filter((comment) => !comment.is_deleted).length
    : null;

  const updateMutation = useUpdateTicket(groupId, ticket.id);
  const deleteMutation = useDeleteTicket(groupId);

  const isClosed = ticket.status === 'closed';

  async function handleDelete() {
    setDeleteError('');
    try {
      await deleteMutation.mutateAsync(ticket.id);
      navigate(`/tickets?group=${groupId}`);
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete issue.'));
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between gap-4">
        <Link
          to={`/tickets?group=${groupId}`}
          className="inline-flex items-center gap-1 text-sm text-slate-400 hover:text-white"
        >
          <ArrowLeft className="h-4 w-4" /> Back to Issues
        </Link>
        {isAdmin && (
          <div className="flex shrink-0 items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5 border border-white/10"
              onClick={() => setIsEditing(true)}
            >
              <Pencil className="h-3.5 w-3.5" /> Edit Issue
            </Button>
            <DropdownMenu.Root>
              <DropdownMenu.Trigger
                aria-label="Issue actions"
                className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white"
              >
                <MoreVertical className="h-4 w-4" />
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content
                  align="end"
                  sideOffset={8}
                  className="z-50 w-40 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50"
                >
                  <DropdownMenu.Item
                    onSelect={() => setIsConfirmingDelete(true)}
                    className="cursor-pointer px-4 py-2 text-sm text-red-400 outline-none transition-colors data-highlighted:bg-white/10"
                  >
                    Delete issue
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>
        )}
      </div>

      <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="flex flex-col gap-6">
          <div className="rounded-xl border border-white/10 bg-white/5 p-6">
            <p className="text-sm text-slate-500">
              <span className="font-medium">#{ticket.ticket_number}</span>
              <span aria-hidden> · </span>
              <span title={formatDateTime(ticket.created_at)}>
                Created {formatRelativeTime(ticket.created_at)}
              </span>
            </p>
            <h1 className="mt-2 text-2xl font-bold text-white">{ticket.title}</h1>
            {/* Preview only — the full description lives in the Details tab. */}
            <p className="mt-2 line-clamp-2 text-sm text-slate-400">{ticket.description}</p>

            <dl className="mt-6 flex flex-wrap items-start gap-x-10 gap-y-4">
              <div>
                <dt className="text-xs font-medium text-slate-500">Status</dt>
                <dd className="mt-1.5">
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
                    <Badge variant={STATUS_VARIANT[ticket.status]}>
                      {capitalize(ticket.status)}
                    </Badge>
                  )}
                </dd>
              </div>
              <div>
                <dt className="text-xs font-medium text-slate-500">Priority</dt>
                <dd className="mt-1.5">
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
                    <Badge variant={PRIORITY_VARIANT[ticket.priority]}>
                      {capitalize(ticket.priority)}
                    </Badge>
                  )}
                </dd>
              </div>
              <div>
                <dt className="text-xs font-medium text-slate-500">Reporter</dt>
                <dd className="mt-1.5 flex items-center gap-2">
                  <Avatar name={ticket.created_by_name} seed={ticket.created_by} />
                  <span className="text-sm text-slate-200">{ticket.created_by_name}</span>
                </dd>
              </div>
              <div>
                <dt className="text-xs font-medium text-slate-500">Team</dt>
                <dd className="mt-1.5 flex items-center gap-2 text-sm text-slate-200">
                  <Users className="h-4 w-4 text-slate-400" />
                  {teamName ? (
                    <Link to={`/groups/${groupId}`} className="hover:text-white hover:underline">
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

          <div className="flex flex-col gap-4">
            {/* The Links tab joins this row once ticket relations exist. */}
            <div role="tablist" className="flex gap-6 border-b border-white/10">
              <TabButton
                icon={FileText}
                isActive={activeTab === 'details'}
                onClick={() => setActiveTab('details')}
              >
                Details
              </TabButton>
              <TabButton
                icon={MessageSquare}
                isActive={activeTab === 'comments'}
                onClick={() => setActiveTab('comments')}
              >
                Comments
                {commentCount !== null && (
                  <span className="rounded-full bg-white/10 px-1.5 py-0.5 text-xs text-slate-300">
                    {commentCount}
                  </span>
                )}
              </TabButton>
            </div>

            {/* Inactive panel is hidden, not unmounted — a half-written comment
                or open reply box survives a switch to Details and back. */}
            <div
              className={
                activeTab === 'details'
                  ? 'rounded-xl border border-white/10 bg-white/5 p-6'
                  : 'hidden'
              }
            >
              <p className="whitespace-pre-wrap text-sm text-slate-200">{ticket.description}</p>
            </div>
            <div
              className={
                activeTab === 'comments'
                  ? 'rounded-xl border border-white/10 bg-white/5 p-6'
                  : 'hidden'
              }
            >
              <CommentList
                groupId={groupId}
                ticketId={ticket.id}
                isAdmin={isAdmin}
                isClosed={isClosed}
              />
            </div>
          </div>
        </div>

        <aside className="flex flex-col gap-6">
          <TicketMeta ticket={ticket} />
          {/* ai/ is a separate, not-yet-built feature (docs/frontend.md lists it
              as part of this page) — the placeholder holds its rail slot. */}
          <div className="rounded-xl border border-white/10 bg-white/5 p-6">
            <h2 className="text-sm font-semibold text-slate-400">AI</h2>
            <p className="mt-2 text-sm text-slate-500">Coming soon.</p>
          </div>
        </aside>
      </div>

      <Modal isOpen={isEditing} onClose={() => setIsEditing(false)} title="Edit issue">
        <EditTicketForm
          groupId={groupId}
          ticket={ticket}
          onSaved={() => setIsEditing(false)}
          onCancel={() => setIsEditing(false)}
        />
      </Modal>

      <Modal
        isOpen={isConfirmingDelete}
        onClose={() => {
          setIsConfirmingDelete(false);
          setDeleteError('');
        }}
        title="Delete issue"
      >
        <p className="text-sm text-slate-300">
          Are you sure you want to delete{' '}
          <span className="font-semibold text-white">
            #{ticket.ticket_number} {ticket.title}
          </span>
          ? This cannot be undone.
        </p>

        {deleteError && <p className="mt-3 text-sm text-red-500">{deleteError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button
            variant="ghost"
            onClick={() => {
              setIsConfirmingDelete(false);
              setDeleteError('');
            }}
          >
            Cancel
          </Button>
          <Button variant="danger" disabled={deleteMutation.isPending} onClick={handleDelete}>
            {deleteMutation.isPending ? 'Deleting…' : 'Delete issue'}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
