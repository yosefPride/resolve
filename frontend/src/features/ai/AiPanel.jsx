import * as Dialog from '@radix-ui/react-dialog';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { ChevronDown, Maximize2, Minimize2, Send, SquarePen, Trash2 } from 'lucide-react';
import { useState } from 'react';
import brandMark from '../../assets/brand-mark.svg';
import Avatar from '../../components/ui/Avatar';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Modal from '../../components/ui/Modal';
import { useAuth } from '../../hooks/useAuth';
import {
  useConversationMessages,
  useConversations,
  useCreateConversation,
  useDeleteConversation,
  useSendConversationMessage,
  useTicketAnalysis,
  useTicketSummary,
} from '../../hooks/useAI';
import { errorMessage } from '../../utils/errors';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import AiInsightCard from './AiInsightCard';

// Same cap as the backend's MAX_MESSAGE_LEN in ai/handlers.rs — used here only
// to block obviously-too-long input client-side; the backend remains the
// source of truth and still validates on submit.
const MAX_MESSAGE_LEN = 2000;

// Flat, oldest-first list like CommentList — no left/right split by
// "ownership": a conversation is private to one user, so every user message
// in it is the same person's, and only role (user vs. assistant) needs to
// read visually distinct. The assistant gets the brand mark in place of an
// Avatar and a tinted bubble so it reads as distinct at a glance.
function ChatMessageBubble({ message }) {
  const isAssistant = message.role === 'assistant';
  return (
    <div className="flex flex-col gap-1">
      <p className="flex items-center gap-2 text-xs text-slate-500">
        {isAssistant ? (
          <img src={brandMark} alt="" className="h-5 w-5 opacity-70" />
        ) : (
          <Avatar name={message.user_name} seed={message.user_id} size="sm" />
        )}
        <span>
          {isAssistant ? 'Assistant' : message.user_name} ·{' '}
          <span title={formatDateTime(message.created_at)}>
            {formatRelativeTime(message.created_at)}
          </span>
        </span>
      </p>
      <p
        className={`whitespace-pre-wrap break-words rounded-lg px-3 py-2 text-xs leading-relaxed ${
          isAssistant ? 'bg-black/30 text-slate-300' : 'bg-sky-500/10 text-slate-200'
        }`}
      >
        {message.content}
      </p>
    </div>
  );
}

// Shown in place of the assistant's reply between send and response — the
// same brand mark used for a real assistant message, animated so it reads
// as "working" rather than a static/frozen icon, plus a classic three-dot
// typing indicator inside the bubble.
function PendingAssistantBubble() {
  return (
    <div className="flex flex-col gap-1">
      <p className="flex items-center gap-2 text-xs text-slate-500">
        <img src={brandMark} alt="" className="h-5 w-5 animate-pulse opacity-70" />
        <span>Assistant</span>
      </p>
      <div className="flex w-fit items-center gap-1 rounded-lg bg-black/30 px-3 py-2.5">
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-slate-500 [animation-delay:-0.3s]" />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-slate-500 [animation-delay:-0.15s]" />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-slate-500" />
      </div>
    </div>
  );
}

const PILL =
  'rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-60';

// Dropdown trigger + list for switching between the caller's own conversations
// on this ticket. Each row's delete button is a plain sibling button next to
// the DropdownMenu.Item, not nested inside it — nesting an interactive
// element inside a Radix menu item means fighting its own click/keyboard
// selection handling; keeping the Item to just the label sidesteps that
// entirely.
function ConversationSwitcher({
  conversations,
  activeConversationId,
  onSelectConversation,
  onRequestDelete,
}) {
  const activeConversation = conversations.find((c) => c.id === activeConversationId);

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger className="flex min-w-0 items-center gap-1 text-sm font-semibold text-slate-300 outline-none hover:text-white">
        <span className="truncate">{activeConversation?.title || 'New chat'}</span>
        <ChevronDown className="h-3.5 w-3.5 shrink-0 text-slate-500" />
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={8}
          className="z-50 max-h-80 w-64 overflow-y-auto rounded-lg border border-white/10 bg-neutral-800 py-1 shadow-2xl shadow-black/50"
        >
          {conversations.length === 0 && (
            <p className="px-4 py-3 text-xs text-slate-500">No conversations yet.</p>
          )}

          {conversations.map((conversation) => (
            <div key={conversation.id} className="flex items-center gap-1 pr-1 pl-1">
              <DropdownMenu.Item
                onSelect={() => onSelectConversation(conversation.id)}
                className={`flex-1 cursor-pointer truncate rounded px-3 py-2 text-sm outline-none transition-colors data-highlighted:bg-white/10 ${
                  conversation.id === activeConversationId ? 'text-white' : 'text-slate-300'
                }`}
              >
                {conversation.title || 'New chat'}
              </DropdownMenu.Item>
              <button
                type="button"
                onClick={() => onRequestDelete(conversation.id)}
                aria-label="Delete conversation"
                title="Delete conversation"
                className="shrink-0 rounded p-1.5 text-slate-500 transition-colors hover:bg-white/10 hover:text-red-400"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

// The full chat UI, mounted independently by AiPanel for the inline rail and
// (when expanded) again inside a Dialog. The two mounts don't share local
// draft/error/confirm state, but activeConversationId is lifted to AiPanel
// (see its doc comment) so both mounts always show the same conversation, and
// useConversations/useConversationMessages/useTicketSummary/useTicketAnalysis
// key on the same tuples, so React Query's cache keeps the actual data in
// sync between them for free: send from the expanded view and the rail
// updates too, no prop-drilling of the data itself required.
function ChatPanel({
  ticket,
  groupId,
  containerClassName,
  isExpandedView = false,
  onToggleExpand,
  activeConversationId,
  onSelectConversation,
}) {
  const { user } = useAuth();
  const conversationsQuery = useConversations(groupId, ticket.id);
  const chatQuery = useConversationMessages(groupId, ticket.id, activeConversationId);
  const summaryQuery = useTicketSummary(groupId, ticket.id);
  const analysisQuery = useTicketAnalysis(groupId, ticket.id);
  const createConversation = useCreateConversation(groupId, ticket.id);
  const sendMessage = useSendConversationMessage(groupId, ticket.id);
  const deleteConversation = useDeleteConversation(groupId, ticket.id);

  const [draft, setDraft] = useState('');
  const [sendError, setSendError] = useState('');
  const [confirmingDeleteId, setConfirmingDeleteId] = useState(null);
  const [deleteError, setDeleteError] = useState('');
  // The just-sent message, shown as its own bubble (plus a pending-assistant
  // placeholder) the instant Send is clicked — otherwise nothing on screen
  // changes until the whole round trip (persist + Gemini call) resolves,
  // which reads as the UI being unresponsive. Cleared once
  // sendMessage.mutateAsync settles; useSendConversationMessage's onSuccess
  // awaits its cache invalidation, so the real messages are already in the
  // cache by then and there's no gap between this disappearing and them
  // appearing.
  const [pendingMessage, setPendingMessage] = useState(null);

  const conversations = conversationsQuery.data ?? [];
  const messages = chatQuery.data ?? [];
  // [...draft].length, not draft.length, to count Unicode code points like
  // the backend's .chars().count() — see CommentForm's contentLength.
  const isTooLong = [...draft].length > MAX_MESSAGE_LEN;
  const canSend = draft.trim() !== '' && !isTooLong && !sendMessage.isPending;

  // No active conversation yet (brand-new ticket, or nothing selected) isn't
  // a blocked state — sending creates one first, then sends into it, so the
  // composer stays enabled exactly like it already was before conversations
  // existed at all.
  async function handleSend(event) {
    event.preventDefault();
    if (!canSend) return;
    setSendError('');
    const content = draft.trim();
    // Cleared and shown as a pending bubble immediately, before the request
    // even starts — see pendingMessage's doc comment.
    setDraft('');
    setPendingMessage(content);
    try {
      let conversationId = activeConversationId;
      if (!conversationId) {
        const created = await createConversation.mutateAsync();
        conversationId = created.id;
        onSelectConversation(conversationId);
      }
      await sendMessage.mutateAsync({ conversationId, message: content });
    } catch (err) {
      // No special-casing for 429: ApiError::RateLimited's message ("chat
      // message limit reached (N per hour) — try again later") is already
      // specific, and errorMessage surfaces it as-is — see errors/api_error.rs
      // and ai/service.rs's CHAT_RATE_LIMIT check.
      setSendError(errorMessage(err, "Couldn't send that message."));
      // Restore the draft so a failed send doesn't lose what was typed.
      setDraft(content);
    } finally {
      setPendingMessage(null);
    }
  }

  // Unlike the old single-shared-thread's "New chat" (which destructively
  // wiped the thread and needed a confirm modal), creating a conversation
  // adds a new one alongside any existing ones — nothing to confirm.
  async function handleNewChat() {
    setSendError('');
    try {
      const created = await createConversation.mutateAsync();
      onSelectConversation(created.id);
    } catch (err) {
      setSendError(errorMessage(err, "Couldn't start a new chat."));
    }
  }

  async function handleDeleteConversation() {
    setDeleteError('');
    try {
      await deleteConversation.mutateAsync(confirmingDeleteId);
      // Deleting the active conversation falls back to the empty state
      // rather than guessing which other conversation the user meant next.
      if (confirmingDeleteId === activeConversationId) {
        onSelectConversation(null);
      }
      setConfirmingDeleteId(null);
    } catch (err) {
      setDeleteError(errorMessage(err, "Couldn't delete that conversation."));
    }
  }

  // isLoading (not isFetching): true only for the first fetch of a
  // conversation that has no cached data yet, not for the background refetch
  // that follows sending a message — that keeps showing the existing list
  // until the new one lands, same as any other query-backed list here.
  const isChatLoading = chatQuery.isLoading;

  const hasActivity =
    messages.length > 0 ||
    pendingMessage !== null ||
    chatQuery.isError ||
    summaryQuery.data ||
    summaryQuery.error ||
    summaryQuery.isFetching ||
    analysisQuery.data ||
    analysisQuery.error ||
    analysisQuery.isFetching;

  return (
    <div className={containerClassName}>
      <div className="flex items-center justify-between gap-2 border-b border-white/10 px-4 py-3">
        <ConversationSwitcher
          conversations={conversations}
          activeConversationId={activeConversationId}
          onSelectConversation={onSelectConversation}
          onRequestDelete={setConfirmingDeleteId}
        />
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={onToggleExpand}
            aria-label={isExpandedView ? 'Collapse' : 'Expand'}
            title={isExpandedView ? 'Collapse' : 'Expand'}
            className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isExpandedView ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
          </button>
          <button
            type="button"
            disabled={createConversation.isPending}
            onClick={handleNewChat}
            aria-label="New chat"
            title="Start a new chat"
            className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            <SquarePen className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* items-center/justify-center only while there's nothing that can
          overflow (loading or empty). Centering a flex column that *can*
          overflow makes the browser split the overflow evenly above and
          below the content, and an overflow-y-auto container can only ever
          scroll toward the end — so the portion pushed above the visible
          area becomes permanently unreachable. Once real content exists,
          top-align instead so scrolling reaches the actual first message. */}
      <div
        className={`flex flex-1 flex-col gap-4 overflow-y-auto px-6 py-6 text-center ${
          isChatLoading || !hasActivity ? 'items-center justify-center' : ''
        }`}
      >
        {isChatLoading && <p className="text-xs text-slate-500">Loading chat…</p>}

        {!isChatLoading && !hasActivity && (
          <>
            <img src={brandMark} alt="" className="h-12 w-12 opacity-10" />
            <p className="text-xs text-slate-500">
              Ask about this issue, or try Summarize or Analyze below.
            </p>
          </>
        )}

        {!isChatLoading && hasActivity && (
          <div className="flex w-full flex-col gap-3 text-left">
            {chatQuery.isError && (
              <p className="text-xs text-red-400">
                {errorMessage(chatQuery.error, "Couldn't load chat history.")}
              </p>
            )}

            {messages.map((message) => (
              <ChatMessageBubble key={message.id} message={message} />
            ))}

            {pendingMessage !== null && (
              <>
                <ChatMessageBubble
                  message={{
                    id: 'pending',
                    role: 'user',
                    content: pendingMessage,
                    user_name: user.name,
                    user_id: user.id,
                    created_at: new Date().toISOString(),
                  }}
                />
                <PendingAssistantBubble />
              </>
            )}

            {(summaryQuery.data || summaryQuery.error || summaryQuery.isFetching) && (
              <AiInsightCard
                title="Summary"
                isLoading={summaryQuery.isFetching}
                error={summaryQuery.error && errorMessage(summaryQuery.error, "Couldn't generate a summary.")}
                cached={summaryQuery.data?.cached}
              >
                {summaryQuery.data?.summary}
              </AiInsightCard>
            )}

            {(analysisQuery.data || analysisQuery.error || analysisQuery.isFetching) && (
              <AiInsightCard
                title="Analysis"
                isLoading={analysisQuery.isFetching}
                error={analysisQuery.error && errorMessage(analysisQuery.error, "Couldn't generate an analysis.")}
                cached={analysisQuery.data?.cached}
              >
                <dl className="flex flex-col gap-1.5">
                  <div>
                    <dt className="inline font-medium text-slate-400">Severity: </dt>
                    <dd className="inline">{analysisQuery.data?.severity_prediction}</dd>
                  </div>
                  <div>
                    <dt className="inline font-medium text-slate-400">Classification: </dt>
                    <dd className="inline">{analysisQuery.data?.classification}</dd>
                  </div>
                  <div>
                    <dt className="inline font-medium text-slate-400">Suggested fix: </dt>
                    <dd className="inline">{analysisQuery.data?.suggested_fix}</dd>
                  </div>
                </dl>
              </AiInsightCard>
            )}
          </div>
        )}

        <div className="flex flex-wrap justify-center gap-2">
          <button
            type="button"
            disabled={summaryQuery.isFetching}
            onClick={() => summaryQuery.refetch()}
            className={PILL}
          >
            {summaryQuery.isFetching ? 'Summarizing…' : 'Summarize issue'}
          </button>
          <button
            type="button"
            disabled={analysisQuery.isFetching}
            onClick={() => analysisQuery.refetch()}
            className={PILL}
          >
            {analysisQuery.isFetching ? 'Analyzing…' : 'Analyze issue'}
          </button>
        </div>
      </div>

      <div className="border-t border-white/10 bg-black px-4 py-2">
        <p className="truncate text-xs text-slate-400">{ticket.title}</p>
      </div>

      <form onSubmit={handleSend} className="border-t border-white/10 p-4">
        {sendError && <p className="mb-2 text-xs text-red-400">{sendError}</p>}
        <div className="flex items-center gap-2">
          <Input
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            disabled={sendMessage.isPending || createConversation.isPending}
            placeholder="Ask about this issue…"
            className="flex-1 text-sm"
          />
          <button
            type="submit"
            disabled={!canSend}
            aria-label="Send message"
            className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400 transition-colors hover:bg-white/20 hover:text-white disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-white/10 disabled:hover:text-slate-400"
          >
            <Send className="h-4 w-4" />
          </button>
        </div>
      </form>

      <Modal
        isOpen={confirmingDeleteId !== null}
        onClose={() => {
          setConfirmingDeleteId(null);
          setDeleteError('');
        }}
        title="Delete conversation"
      >
        <p className="text-sm text-slate-300">
          This permanently deletes this conversation and its messages. This cannot be undone.
        </p>

        {deleteError && <p className="mt-3 text-sm text-red-500">{deleteError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button
            variant="ghost"
            onClick={() => {
              setConfirmingDeleteId(null);
              setDeleteError('');
            }}
          >
            Cancel
          </Button>
          <Button variant="danger" disabled={deleteConversation.isPending} onClick={handleDeleteConversation}>
            {deleteConversation.isPending ? 'Deleting…' : 'Delete conversation'}
          </Button>
        </div>
      </Modal>
    </div>
  );
}

// Fixed rail height (h-132 = 33rem: the gap-4 plus the h-128 Details/Comments
// panel in TicketDetail, so the rail bottom lines up with that panel's
// bottom) rather than flex-1 stretch-to-sibling: a long summary or analysis
// result scrolls inside the panel instead of growing it, and the height
// stays constant instead of depending on TicketMeta's height (which only
// exists as a stretch target at lg+ anyway — there's no sibling to stretch
// against once the layout stacks on mobile).
//
// The expanded view is a plain Dialog (not the shared Modal component) since
// Modal is built for a centered, text-only confirmation (fixed max-w-md, p-6
// padding, title-first layout) and this needs a large flex column that ChatPanel
// itself controls the inside of — header, scrolling thread, and composer
// pinned top/bottom. Overlay/content styling is kept visually consistent with
// Modal's rather than factored out, since it's the only other Dialog user.
export default function AiPanel({ ticket, groupId }) {
  const [isExpanded, setIsExpanded] = useState(false);

  // activeConversationId is lifted here (rather than living inside ChatPanel)
  // because it's "which conversation am I looking at", not in-progress input
  // — a user who switches to an older conversation in the rail should see
  // that same conversation on Expand, not silently jump back to most-recent.
  // Everything else (draft text, send/delete errors, the delete-confirm
  // modal) stays local to each ChatPanel mount, same as before.
  //
  // selection has three states, not two: undefined means "no explicit choice
  // yet — auto-follow the most-recently-active conversation" (derived below,
  // purely at render time, rather than synced via an effect); null means
  // "explicitly no conversation" (e.g. right after deleting the active one —
  // it should NOT immediately re-derive to the next-most-recent); an id means
  // "the user picked this one". TicketDetail keys <AiPanel> on ticket.id, so
  // navigating to a different ticket remounts this component and resets
  // selection to undefined — no ticket-change effect needed either.
  const [selection, setSelection] = useState(undefined);
  const conversationsQuery = useConversations(groupId, ticket.id);
  const activeConversationId =
    selection === undefined ? (conversationsQuery.data?.[0]?.id ?? null) : selection;

  return (
    <>
      <ChatPanel
        ticket={ticket}
        groupId={groupId}
        containerClassName="flex h-132 flex-col rounded-xl border border-white/10 bg-white/5"
        onToggleExpand={() => setIsExpanded(true)}
        activeConversationId={activeConversationId}
        onSelectConversation={setSelection}
      />

      <Dialog.Root open={isExpanded} onOpenChange={setIsExpanded}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm" />
          <Dialog.Content
            aria-describedby={undefined}
            className="fixed left-1/2 top-1/2 z-50 flex h-[calc(100%-4rem)] w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border border-white/10 bg-neutral-950 shadow-2xl shadow-black/50"
          >
            <Dialog.Title className="sr-only">Chat</Dialog.Title>
            <ChatPanel
              ticket={ticket}
              groupId={groupId}
              containerClassName="flex h-full flex-col"
              isExpandedView
              onToggleExpand={() => setIsExpanded(false)}
              activeConversationId={activeConversationId}
              onSelectConversation={setSelection}
            />
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </>
  );
}
