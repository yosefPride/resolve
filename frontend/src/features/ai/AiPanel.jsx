import * as Dialog from '@radix-ui/react-dialog';
import { Maximize2, Minimize2, Send, SquarePen } from 'lucide-react';
import { useState } from 'react';
import brandMark from '../../assets/brand-mark.svg';
import Avatar from '../../components/ui/Avatar';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Modal from '../../components/ui/Modal';
import {
  useChatMessages,
  useClearChat,
  useSendChatMessage,
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
// "ownership", since the thread is shared across the group and who asked
// matters as much for a teammate's message as for your own. The assistant
// gets the brand mark in place of an Avatar and a tinted bubble so it reads
// as distinct from any group member at a glance.
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

const PILL =
  'rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-60';

// The full chat UI, mounted independently by AiPanel for the inline rail and
// (when expanded) again inside a Dialog. The two mounts don't share any local
// state — each gets its own draft/confirm/error state — but useChatMessages,
// useTicketSummary, and useTicketAnalysis key on the same [name, groupId,
// ticketId] tuples, so React Query's cache keeps the actual data (messages,
// summary, analysis) in sync between them for free: send from the expanded
// view and the rail updates too, no prop-drilling required.
function ChatPanel({ ticket, groupId, containerClassName, isExpandedView = false, onToggleExpand }) {
  const chatQuery = useChatMessages(groupId, ticket.id);
  const summaryQuery = useTicketSummary(groupId, ticket.id);
  const analysisQuery = useTicketAnalysis(groupId, ticket.id);
  const sendMessage = useSendChatMessage(groupId, ticket.id);
  const clearChat = useClearChat(groupId, ticket.id);

  const [draft, setDraft] = useState('');
  const [sendError, setSendError] = useState('');
  const [confirmingClear, setConfirmingClear] = useState(false);
  const [clearError, setClearError] = useState('');

  const messages = chatQuery.data ?? [];
  // [...draft].length, not draft.length, to count Unicode code points like
  // the backend's .chars().count() — see CommentForm's contentLength.
  const isTooLong = [...draft].length > MAX_MESSAGE_LEN;
  const canSend = draft.trim() !== '' && !isTooLong && !sendMessage.isPending;

  async function handleSend(event) {
    event.preventDefault();
    if (!canSend) return;
    setSendError('');
    try {
      await sendMessage.mutateAsync(draft.trim());
      setDraft('');
    } catch (err) {
      // No special-casing for 429: ApiError::RateLimited's message ("chat
      // message limit reached (N per hour) — try again later") is already
      // specific, and errorMessage surfaces it as-is — see errors/api_error.rs
      // and ai/service.rs's CHAT_RATE_LIMIT check.
      setSendError(errorMessage(err, "Couldn't send that message."));
    }
  }

  async function handleClearChat() {
    setClearError('');
    try {
      await clearChat.mutateAsync();
      setConfirmingClear(false);
    } catch (err) {
      setClearError(errorMessage(err, "Couldn't start a new chat."));
    }
  }

  // isLoading (not isFetching): true only for the first fetch of a ticket that
  // has no cached data yet, not for the background refetch that follows
  // sending a message or clearing the chat — those keep showing the existing
  // list until the new one lands, same as any other query-backed list here.
  const isChatLoading = chatQuery.isLoading;

  const hasActivity =
    messages.length > 0 ||
    chatQuery.isError ||
    summaryQuery.data ||
    summaryQuery.error ||
    summaryQuery.isFetching ||
    analysisQuery.data ||
    analysisQuery.error ||
    analysisQuery.isFetching;

  return (
    <div className={containerClassName}>
      <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
        <span className="text-sm font-semibold text-slate-300">Chats</span>
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
            disabled={messages.length === 0 || clearChat.isPending}
            onClick={() => setConfirmingClear(true)}
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
            disabled={sendMessage.isPending}
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
        isOpen={confirmingClear}
        onClose={() => {
          setConfirmingClear(false);
          setClearError('');
        }}
        title="Start a new chat"
      >
        <p className="text-sm text-slate-300">
          This clears the chat history for this issue for everyone in the group. This cannot be
          undone.
        </p>

        {clearError && <p className="mt-3 text-sm text-red-500">{clearError}</p>}

        <div className="mt-6 flex justify-end gap-3">
          <Button
            variant="ghost"
            onClick={() => {
              setConfirmingClear(false);
              setClearError('');
            }}
          >
            Cancel
          </Button>
          <Button variant="danger" disabled={clearChat.isPending} onClick={handleClearChat}>
            {clearChat.isPending ? 'Clearing…' : 'Start new chat'}
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

  return (
    <>
      <ChatPanel
        ticket={ticket}
        groupId={groupId}
        containerClassName="flex h-132 flex-col rounded-xl border border-white/10 bg-white/5"
        onToggleExpand={() => setIsExpanded(true)}
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
            />
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </>
  );
}
