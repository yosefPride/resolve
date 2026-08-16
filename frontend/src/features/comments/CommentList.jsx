import { useEffect, useRef, useState } from 'react';
import { useComments, useDeleteComment, useRemoveReaction, useSetReaction } from '../../hooks/useComments';
import { useAuth } from '../../hooks/useAuth';
import { errorMessage } from '../../utils/errors';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import Avatar from '../../components/ui/Avatar';
import Button from '../../components/ui/Button';
import EmojiPicker from '../../components/ui/EmojiPicker';
import Modal from '../../components/ui/Modal';
import CommentForm from './CommentForm';

// The API hands back one flat array, oldest-first. Map preserves insertion
// order, so both roots and each reply list come out oldest-first with no sort.
function buildCommentTree(comments) {
  const byId = new Map(comments.map((comment) => [comment.id, { ...comment, replies: [] }]));
  const roots = [];

  for (const node of byId.values()) {
    const parent = node.parent_comment_id ? byId.get(node.parent_comment_id) : null;

    // Every reply shows which comment it answers, at any depth — not just past
    // MAX_INDENT_DEPTH. Copied into a flat summary rather than holding the
    // parent node itself, to keep the tree free of back-references.
    if (!node.parent_comment_id) {
      node.parentSummary = null;
    } else if (parent) {
      node.parentSummary = {
        userName: parent.user_name,
        content: parent.content,
        isDeleted: parent.is_deleted,
        missing: false,
      };
    } else {
      node.parentSummary = { userName: null, content: null, isDeleted: false, missing: true };
    }

    if (parent) {
      parent.replies.push(node);
    } else {
      // A real top-level comment, or a reply whose parent isn't in the payload.
      // The second case shouldn't normally arise — a comment with replies gets
      // tombstoned instead of removed — but delete_comment's has_replies check
      // and the delete that follows it aren't atomic, so a reply created in
      // that window can point at an id that's already gone. Treating it as a
      // root keeps it on screen instead of silently dropping it.
      roots.push(node);
    }
  }

  return roots;
}

// WhatsApp-style reply header: the author and first line of the comment being
// answered, so a reply still reads in context once indentation stops growing.
function QuotedParent({ summary }) {
  if (summary.missing) {
    return (
      <div className="mb-2 rounded-md border-l-2 border-white/20 bg-white/5 px-2 py-1">
        <p className="text-xs italic text-slate-500">In reply to a comment that was deleted</p>
      </div>
    );
  }

  return (
    <div className="mb-2 flex flex-col gap-0.5 rounded-md border-l-2 border-sky-400/50 bg-white/5 px-2 py-1">
      <p className="text-xs font-medium text-sky-300">{summary.userName}</p>
      <p
        className={`line-clamp-1 text-xs ${
          summary.isDeleted ? 'italic text-slate-500' : 'text-slate-400'
        }`}
      >
        {summary.content}
      </p>
    </div>
  );
}

// Pills for existing reactions plus an "add reaction" picker, meant to sit
// inline in the same row as the Reply/Delete buttons (not a separate row
// above them) — a Fragment rather than its own flex container, so its pills
// lay out as direct siblings of those buttons. Clicking a pill you're already
// behind on removes it; clicking someone else's pill joins it as your own —
// the backend allows only one reaction per user per comment, so that join
// silently replaces whatever emoji you'd picked before, same as picking a
// fresh one from the picker. Not gated on isClosed: reactions stay allowed on
// a closed ticket, unlike posting a new comment or reply.
function ReactionBar({ comment, onSetReaction, onRemoveReaction }) {
  return (
    <>
      {comment.reactions.map((reaction) => (
        <button
          key={reaction.emoji}
          type="button"
          onClick={() =>
            reaction.reacted_by_me
              ? onRemoveReaction(comment.id)
              : onSetReaction(comment.id, reaction.emoji)
          }
          className={`flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs transition-colors ${
            reaction.reacted_by_me
              ? 'border-sky-400/30 bg-sky-500/10 text-sky-300'
              : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'
          }`}
        >
          <span>{reaction.emoji}</span>
          <span>{reaction.count}</span>
        </button>
      ))}
      <EmojiPicker
        closeOnSelect
        onSelect={(char) => onSetReaction(comment.id, char)}
        triggerClassName="p-1"
      />
    </>
  );
}

function CommentItem({
  comment,
  groupId,
  ticketId,
  currentUserId,
  isAdmin,
  isClosed,
  replyingTo,
  onReply,
  onCancelReply,
  onRequestDelete,
  onSetReaction,
  onRemoveReaction,
}) {
  // Author or Group Admin, matching RbacService::require_owner_or_group_admin.
  // UX only — the backend rejects an unauthorised DELETE either way. A
  // tombstone has nothing left to delete.
  const canDelete = !comment.is_deleted && (comment.user_id === currentUserId || isAdmin);
  const isReplying = replyingTo === comment.id;
  const isOwn = comment.user_id === currentUserId;

  return (
    <li>
      <article
        className={`flex w-[60%] flex-col gap-1 py-2 ${isOwn ? 'ml-auto items-end' : 'mr-auto items-start'}`}
      >
        <p className="flex items-center gap-2 text-xs text-slate-500">
          <Avatar name={comment.user_name} seed={comment.user_id} size="sm" />
          <span>
            {comment.user_name} ·{' '}
            <span title={formatDateTime(comment.created_at)}>
              {formatRelativeTime(comment.created_at)}
            </span>
          </span>
        </p>

        <div
          className={`w-full rounded-2xl px-3 py-2 ${
            isOwn ? 'rounded-tr-sm bg-sky-600/30' : 'rounded-tl-sm bg-white/10'
          }`}
        >
          {comment.parentSummary && <QuotedParent summary={comment.parentSummary} />}

          {/* Plain text, never HTML: the backend stores comment content verbatim
              without sanitising it, so React's automatic escaping here is the
              only thing between a pasted <script> and the DOM. Do not swap this
              for dangerouslySetInnerHTML or a markdown renderer. */}
          <p
            className={`whitespace-pre-wrap break-words text-sm ${
              comment.is_deleted ? 'italic text-slate-500' : 'text-slate-200'
            }`}
          >
            {comment.content}
          </p>
        </div>

        {!comment.is_deleted && (
          <div className="flex flex-wrap items-center gap-1">
            <ReactionBar
              comment={comment}
              onSetReaction={onSetReaction}
              onRemoveReaction={onRemoveReaction}
            />
            {!isClosed && (
              <Button variant="ghost" size="sm" onClick={() => onReply(comment.id)}>
                Reply
              </Button>
            )}
            {canDelete && (
              <Button
                variant="ghost"
                size="sm"
                className="text-red-400 hover:text-red-300"
                onClick={() => onRequestDelete(comment)}
              >
                Delete
              </Button>
            )}
          </div>
        )}
      </article>

      {isReplying && (
        <div className="pb-2">
          <CommentForm
            groupId={groupId}
            ticketId={ticketId}
            parentCommentId={comment.id}
            autoFocus
            onDone={onCancelReply}
            onCancel={onCancelReply}
          />
        </div>
      )}

      {comment.replies.length > 0 && (
        <ul>
          {comment.replies.map((reply) => (
            <CommentItem
              key={reply.id}
              comment={reply}
              groupId={groupId}
              ticketId={ticketId}
              currentUserId={currentUserId}
              isAdmin={isAdmin}
              isClosed={isClosed}
              replyingTo={replyingTo}
              onReply={onReply}
              onCancelReply={onCancelReply}
              onRequestDelete={onRequestDelete}
              onSetReaction={onSetReaction}
              onRemoveReaction={onRemoveReaction}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

export default function CommentList({ groupId, ticketId, isAdmin, isClosed, isVisible = true }) {
  const { user } = useAuth();
  const { data: comments, status, error } = useComments(groupId, ticketId);
  const [replyingTo, setReplyingTo] = useState(null);
  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleteError, setDeleteError] = useState('');
  const deleteComment = useDeleteComment(groupId, ticketId);
  const setReaction = useSetReaction(groupId, ticketId);
  const removeReaction = useRemoveReaction(groupId, ticketId);
  const threadRef = useRef(null);

  // Comments are oldest-first, so the latest is at the bottom — jump there
  // whenever the thread (re)loads or gains a comment. isVisible matters
  // because the tab panel hides via display:none: scrollHeight is 0 while
  // hidden, so the scroll must re-run when the tab actually shows.
  useEffect(() => {
    if (!isVisible) return;
    const node = threadRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [comments, isVisible]);

  async function handleDelete() {
    setDeleteError('');
    try {
      await deleteComment.mutateAsync(pendingDelete.id);
      setPendingDelete(null);
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete comment.'));
    }
  }

  function closeDeleteModal() {
    setPendingDelete(null);
    setDeleteError('');
  }

  if (status === 'pending') {
    return <p className="text-sm text-slate-400">Loading comments…</p>;
  }

  if (status === 'error') {
    return <p className="text-sm text-red-500">{errorMessage(error, "Couldn't load comments.")}</p>;
  }

  const tree = buildCommentTree(comments);
  // Someone else can close the ticket while a reply box is open; drop the open
  // box on the refetch rather than leaving a form that can only ever 409.
  const activeReplyingTo = isClosed ? null : replyingTo;

  return (
    // h-full + the scrollable flex-1 wrapper let a fixed-height parent (the
    // comments tab panel) keep the composer pinned while only the thread
    // scrolls; in an unconstrained parent it lays out exactly as before.
    <div className="flex h-full min-h-0 flex-col gap-4">
      <div ref={threadRef} className="min-h-0 flex-1 overflow-y-auto">
        {tree.length === 0 ? (
          <p className="text-sm text-slate-500">No comments yet.</p>
        ) : (
          <ul>
            {tree.map((comment) => (
              <CommentItem
                key={comment.id}
                comment={comment}
                groupId={groupId}
                ticketId={ticketId}
                currentUserId={user.id}
                isAdmin={isAdmin}
                isClosed={isClosed}
                replyingTo={activeReplyingTo}
                onReply={setReplyingTo}
                onCancelReply={() => setReplyingTo(null)}
                onRequestDelete={setPendingDelete}
                onSetReaction={(commentId, emoji) => setReaction.mutate({ commentId, emoji })}
                onRemoveReaction={(commentId) => removeReaction.mutate(commentId)}
              />
            ))}
          </ul>
        )}
      </div>

      {/* A closed ticket is read-only for new comments (the backend answers
          POST with 409), so the composer is removed outright rather than
          disabled — there's nothing useful to click. */}
      {!isClosed && <CommentForm groupId={groupId} ticketId={ticketId} />}

      <Modal isOpen={pendingDelete !== null} onClose={closeDeleteModal} title="Delete comment">
        <p className="text-sm text-slate-300">
          Are you sure you want to delete this comment? This cannot be undone.
        </p>

        {deleteError && <p className="mt-3 text-sm text-red-500">{deleteError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button variant="ghost" onClick={closeDeleteModal}>
            Cancel
          </Button>
          <Button variant="danger" disabled={deleteComment.isPending} onClick={handleDelete}>
            {deleteComment.isPending ? 'Deleting…' : 'Delete comment'}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
