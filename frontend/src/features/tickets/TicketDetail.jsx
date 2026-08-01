import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDeleteTicket, useUpdateTicket } from '../../hooks/useTickets';
import { errorMessage } from '../../utils/errors';
import { formatDateTime } from '../../utils/format';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
import CommentList from '../comments/CommentList';
import EditTicketForm from './EditTicketForm';

const PRIORITY_VARIANT = {
  low: 'neutral',
  high: 'accent',
  critical: 'danger',
};

const STATUS_VARIANT = {
  open: 'accent',
  closed: 'outline',
};

function capitalize(value) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function initials(name) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0].toUpperCase())
    .join('');
}

// isAdmin gates Edit/Delete/status-toggle — RBAC is UX-only here, the backend
// rejects a non-Group-Admin PATCH/DELETE regardless (docs/rbac.md).
export default function TicketDetail({ ticket, teamName, groupId, isAdmin }) {
  const navigate = useNavigate();
  const [isEditing, setIsEditing] = useState(false);
  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [deleteError, setDeleteError] = useState('');

  const toggleStatusMutation = useUpdateTicket(groupId, ticket.id);
  const deleteMutation = useDeleteTicket(groupId);

  const isClosed = ticket.status === 'closed';

  function handleToggleStatus() {
    toggleStatusMutation.mutate({ status: isClosed ? 'open' : 'closed' });
  }

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
      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <p className="text-sm text-slate-500">
          <span className="font-medium">#{ticket.ticket_number}</span>
          <span aria-hidden> · </span>
          Created {formatDateTime(ticket.created_at)}
        </p>
        <h1 className="mt-2 text-2xl font-bold text-white">{ticket.title}</h1>
        {/* Preview only — the full description has its own section below. */}
        <p className="mt-2 line-clamp-2 text-sm text-slate-400">{ticket.description}</p>

        <dl className="mt-6 flex flex-wrap items-start gap-x-10 gap-y-4">
          <div>
            <dt className="text-xs font-medium text-slate-500">Status</dt>
            <dd className="mt-1.5">
              <Badge variant={STATUS_VARIANT[ticket.status]}>{capitalize(ticket.status)}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-xs font-medium text-slate-500">Priority</dt>
            <dd className="mt-1.5">
              <Badge variant={PRIORITY_VARIANT[ticket.priority]}>{capitalize(ticket.priority)}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-xs font-medium text-slate-500">Reporter</dt>
            <dd className="mt-1.5 flex items-center gap-2">
              <span className="flex h-6 w-6 items-center justify-center rounded-full bg-white/10 text-[10px] font-semibold text-slate-300">
                {initials(ticket.created_by_name)}
              </span>
              <span className="text-sm text-slate-200">{ticket.created_by_name}</span>
            </dd>
          </div>
          <div>
            <dt className="text-xs font-medium text-slate-500">Team</dt>
            <dd className="mt-1.5 text-sm text-slate-200">{teamName || '—'}</dd>
          </div>
        </dl>

        {isAdmin && (
          <div className="mt-6 flex flex-wrap gap-2 border-t border-white/10 pt-4">
            <Button
              variant="ghost"
              className="border border-white/10"
              disabled={toggleStatusMutation.isPending}
              onClick={handleToggleStatus}
            >
              {isClosed ? 'Reopen issue' : 'Close issue'}
            </Button>
            <Button variant="ghost" className="border border-white/10" onClick={() => setIsEditing(true)}>
              Edit
            </Button>
            <Button variant="dangerOutline" onClick={() => setIsConfirmingDelete(true)}>
              Delete
            </Button>
          </div>
        )}
      </div>

      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <h2 className="text-sm font-semibold text-slate-400">Description</h2>
        <p className="mt-3 whitespace-pre-wrap text-sm text-slate-200">{ticket.description}</p>
      </div>

      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <h2 className="text-sm font-semibold text-slate-400">Comments</h2>
        <div className="mt-4">
          <CommentList
            groupId={groupId}
            ticketId={ticket.id}
            isAdmin={isAdmin}
            isClosed={isClosed}
          />
        </div>
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
