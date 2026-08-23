import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  Activity,
  ArrowLeft,
  FileText,
  Link2,
  MessageSquare,
  MoreVertical,
  Pencil,
} from 'lucide-react';
import { useDeleteTicket } from '../../hooks/useTickets';
import { useComments } from '../../hooks/useComments';
import { errorMessage } from '../../utils/errors';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import Button from '../../components/ui/Button';
import ConfirmModal from '../../components/ui/ConfirmModal';
import DropdownMenu, { DropdownMenuItem } from '../../components/ui/DropdownMenu';
import Modal from '../../components/ui/Modal';
import ActivityList from '../activity/ActivityList';
import AiPanel from '../ai/AiPanel';
import CommentList from '../comments/CommentList';
import LinksPanel from '../links/LinksPanel';
import DescriptionMarkdown from './DescriptionMarkdown';
import EditTicketForm from './EditTicketForm';
import TicketMeta from './TicketMeta';

function TabButton({ isActive, onClick, icon: Icon, disabled, children }) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={isActive}
      disabled={disabled}
      onClick={onClick}
      className={`-mb-px inline-flex shrink-0 items-center gap-2 border-b-2 px-1 pb-3 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:text-slate-400 ${
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
            <DropdownMenu
              trigger={
                <button
                  type="button"
                  aria-label="Issue actions"
                  className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white"
                >
                  <MoreVertical className="h-4 w-4" />
                </button>
              }
            >
              <DropdownMenuItem variant="danger" onSelect={() => setIsConfirmingDelete(true)}>
                Delete issue
              </DropdownMenuItem>
            </DropdownMenu>
          </div>
        )}
      </div>

      {/* flex-col below lg, same as the other app pages — a bare `grid`
          track can be forced wider than the viewport by a wide descendant's
          min-content (CSS Grid's automatic minimum size), which is what was
          pushing content past the frame on mobile. flex-col doesn't have
          that problem: it stretches children down to its own width instead
          of letting one grow the container. At lg+ the whole area is pinned
          to 80vh and items-stretch hands that height to both columns, so the
          page frame stays put and long content scrolls inside a card rather
          than below the fold. Each column has exactly one child that absorbs
          the height — the active tab panel here, AiPanel in the rail (both
          lg:flex-1) — and everything else is shrink-0 so a short viewport
          eats into the scrollable panel instead of clipping the title card,
          the tab strip, or the stats card. Below lg there is no second
          column and no fixed frame: the panels fall back to h-128 and the
          area grows with the page. */}
      <div className="flex flex-col gap-6 lg:grid lg:h-[80vh] lg:grid-cols-[minmax(0,1fr)_320px] lg:items-stretch">
        <div className="flex flex-col gap-6 lg:min-h-0">
          <div className="break-words rounded-xl border border-white/10 bg-white/5 p-6 lg:shrink-0">
            <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 text-sm text-slate-500">
              <p>
                <span className="font-medium">#{ticket.ticket_number}</span>
                <span aria-hidden> · </span>
                <span title={formatDateTime(ticket.created_at)}>
                  Created {formatRelativeTime(ticket.created_at)}
                </span>
              </p>
              <p title={formatDateTime(ticket.updated_at)}>
                Updated {formatRelativeTime(ticket.updated_at)}
              </p>
            </div>
            <h1 className="mt-2 text-2xl font-bold text-white">{ticket.title}</h1>
          </div>

          <div className="flex flex-col gap-4 lg:min-h-0 lg:flex-1">
            <div role="tablist" className="flex shrink-0 gap-6 overflow-x-auto border-b border-white/10">
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
              <TabButton
                icon={Link2}
                isActive={activeTab === 'links'}
                onClick={() => setActiveTab('links')}
              >
                Links
              </TabButton>
              <TabButton
                icon={Activity}
                isActive={activeTab === 'activity'}
                onClick={() => setActiveTab('activity')}
              >
                Activity
              </TabButton>
            </div>

            {/* Inactive panel is hidden, not unmounted — a half-written comment
                or open reply box survives a switch to Details and back. */}
            {/* Fixed-height panels: long descriptions/threads scroll inside the
                card instead of growing the page. */}
            <div
              className={
                activeTab === 'details'
                  ? 'h-128 overflow-y-auto break-words rounded-xl border border-white/10 bg-white/5 p-6 lg:h-auto lg:min-h-0 lg:flex-1'
                  : 'hidden'
              }
            >
              <DescriptionMarkdown>{ticket.description}</DescriptionMarkdown>
            </div>
            <div
              className={
                activeTab === 'comments'
                  ? 'flex h-128 flex-col rounded-xl border border-white/10 bg-white/5 p-6 lg:h-auto lg:min-h-0 lg:flex-1'
                  : 'hidden'
              }
            >
              <CommentList
                groupId={groupId}
                ticketId={ticket.id}
                isAdmin={isAdmin}
                isClosed={isClosed}
                isVisible={activeTab === 'comments'}
              />
            </div>
            <div
              className={
                activeTab === 'activity'
                  ? 'h-128 overflow-y-auto rounded-xl border border-white/10 bg-white/5 p-6 lg:h-auto lg:min-h-0 lg:flex-1'
                  : 'hidden'
              }
            >
              <ActivityList groupId={groupId} ticketId={ticket.id} />
            </div>
            <div
              className={
                activeTab === 'links'
                  ? 'h-128 overflow-y-auto rounded-xl border border-white/10 bg-white/5 p-6 lg:h-auto lg:min-h-0 lg:flex-1'
                  : 'hidden'
              }
            >
              <LinksPanel groupId={groupId} ticketId={ticket.id} isAdmin={isAdmin} />
            </div>
          </div>
        </div>

        <aside className="flex min-h-0 flex-col gap-6">
          <TicketMeta ticket={ticket} teamName={teamName} groupId={groupId} isAdmin={isAdmin} />
          {/* Keyed on ticket.id so AiPanel fully remounts on ticket-to-ticket
              navigation — it resets its own activeConversationId selection
              state that way instead of needing an effect to do it. */}
          <AiPanel key={ticket.id} ticket={ticket} groupId={groupId} />
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

      <ConfirmModal
        isOpen={isConfirmingDelete}
        onClose={() => {
          setIsConfirmingDelete(false);
          setDeleteError('');
        }}
        title="Delete issue"
        confirmLabel="Delete issue"
        pendingLabel="Deleting…"
        isPending={deleteMutation.isPending}
        error={deleteError}
        onConfirm={handleDelete}
      >
        Are you sure you want to delete{' '}
        <span className="font-semibold text-white">
          #{ticket.ticket_number} {ticket.title}
        </span>
        ? This cannot be undone.
      </ConfirmModal>
    </div>
  );
}
