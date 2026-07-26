import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDeleteTicket, useUpdateTicket } from '../../hooks/useTickets';
import { errorMessage } from '../../utils/errors';
import { formatDateTime } from '../../utils/format';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';
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

// isAdmin gates Edit/Delete/status-toggle — RBAC is UX-only here, the backend
// rejects a non-Group-Admin PATCH/DELETE regardless (docs/rbac.md).
export default function TicketDetail({ ticket, groupId, isAdmin }) {
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
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm font-medium text-slate-500">#{ticket.ticket_number}</p>
          <h1 className="text-2xl font-bold text-white">{ticket.title}</h1>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Badge variant={STATUS_VARIANT[ticket.status]}>{capitalize(ticket.status)}</Badge>
          <Badge variant={PRIORITY_VARIANT[ticket.priority]}>{capitalize(ticket.priority)}</Badge>
        </div>
      </div>

      <p className="text-sm text-slate-400">
        Opened by {ticket.created_by_name} · {formatDateTime(ticket.created_at)}
        {ticket.updated_at !== ticket.created_at && <> · Updated {formatDateTime(ticket.updated_at)}</>}
      </p>

      <p className="whitespace-pre-wrap text-sm text-slate-200">{ticket.description}</p>

      {isAdmin && (
        <div className="flex flex-wrap gap-2 border-t border-white/10 pt-4">
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

      {/* comments/ and ai/ are separate, not-yet-built features (docs/frontend.md
          lists both as part of this page) — placeholders hold their layout slot. */}
      <div className="flex flex-col gap-2 border-t border-white/10 pt-6">
        <h2 className="text-sm font-semibold text-slate-400">Comments</h2>
        <p className="text-sm text-slate-500">Coming soon.</p>
      </div>

      <div className="flex flex-col gap-2 border-t border-white/10 pt-6">
        <h2 className="text-sm font-semibold text-slate-400">AI</h2>
        <p className="text-sm text-slate-500">Coming soon.</p>
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
