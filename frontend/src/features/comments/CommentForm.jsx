import { useRef, useState } from 'react';
import { useCreateComment } from '../../hooks/useComments';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import EmojiPicker from '../../components/ui/EmojiPicker';
import Textarea from '../../components/ui/Textarea';

const MAX_CONTENT_LENGTH = 2000;

// Counts Unicode code points, matching the backend's .chars().count(). Plain
// value.length counts UTF-16 units instead, and the two disagree outside the
// BMP: '😀'.length is 2 where the backend counts 1. Same reason there's no
// maxLength on the textarea — it counts UTF-16 units too, so it would cut
// emoji-heavy input off well before the real limit.
function contentLength(value) {
  return [...value].length;
}

// parentCommentId null = new top-level comment on the ticket; set = reply to
// that comment. Never rendered at all for a closed ticket (see CommentList).
export default function CommentForm({
  groupId,
  ticketId,
  parentCommentId = null,
  onDone,
  onCancel,
  autoFocus = false,
}) {
  const [content, setContent] = useState('');
  const [error, setError] = useState('');
  const createComment = useCreateComment(groupId, ticketId);
  const textareaRef = useRef(null);

  const length = contentLength(content);
  const isEmpty = content.trim() === '';
  const isTooLong = length > MAX_CONTENT_LENGTH;
  const isReply = parentCommentId !== null;

  // Inserts at the cursor (falling back to the end if the textarea hasn't
  // mounted yet) rather than always appending, and refocuses afterward so a
  // user can pick several emoji in a row without re-clicking into the field
  // between each one.
  function insertEmoji(char) {
    const node = textareaRef.current;
    const start = node?.selectionStart ?? content.length;
    const end = node?.selectionEnd ?? content.length;
    const next = content.slice(0, start) + char + content.slice(end);
    setContent(next);
    requestAnimationFrame(() => {
      node?.focus();
      const cursor = start + char.length;
      node?.setSelectionRange(cursor, cursor);
    });
  }

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    try {
      await createComment.mutateAsync({ content, parentCommentId });
      setContent('');
      onDone?.();
    } catch (err) {
      // 409 here means the ticket was closed while this form was open. The
      // generic message would leave the user staring at a composer that's
      // visible but permanently failing, with no hint why.
      if (err.response?.status === 409) {
        setError('This issue was closed. Reload the page to see its current state.');
        return;
      }
      setError(errorMessage(err, 'Failed to post comment.'));
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-2">
      <Textarea
        ref={textareaRef}
        value={content}
        onChange={(event) => setContent(event.target.value)}
        rows={isReply ? 2 : 3}
        autoFocus={autoFocus}
        placeholder={isReply ? 'Write a reply…' : 'Add a comment…'}
        className="text-sm"
      />

      {error && <p className="text-sm text-red-500">{error}</p>}

      <div className="flex items-center justify-end gap-3">
        {/* closeOnSelect stays false (the default): picking an emoji here
            should let the user keep adding more without reopening the panel,
            unlike the reaction bar's single-pick picker. */}
        <EmojiPicker onSelect={insertEmoji} />
        <span className={`mr-auto text-xs ${isTooLong ? 'text-red-400' : 'text-slate-500'}`}>
          {length} / {MAX_CONTENT_LENGTH}
        </span>
        {onCancel && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={createComment.isPending}
            onClick={onCancel}
          >
            Cancel
          </Button>
        )}
        <Button type="submit" size="sm" disabled={isEmpty || isTooLong || createComment.isPending}>
          {createComment.isPending ? 'Posting…' : isReply ? 'Reply' : 'Comment'}
        </Button>
      </div>
    </form>
  );
}
