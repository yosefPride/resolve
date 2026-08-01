import { Maximize2, Send, SquarePen } from 'lucide-react';
import brandMark from '../../assets/brand-mark.svg';
import Input from '../../components/ui/Input';

const PILL =
  'rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-slate-300 transition-colors disabled:cursor-not-allowed disabled:opacity-60';

// Chatbot-shaped placeholder for the not-yet-built AI feature (docs/
// specification/api.md: POST .../summarize, .../analyze). Grows to fill
// whatever height TicketMeta leaves in the stretched rail, so the rail's
// total height matches the main column. Everything is disabled until those
// endpoints exist.
export default function AiPanel({ ticket }) {
  return (
    <div className="flex min-h-0 flex-1 flex-col rounded-xl border border-white/10 bg-white/5">
      <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
        <span className="text-sm font-semibold text-slate-300">Chats</span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            disabled
            aria-label="Expand"
            className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Maximize2 className="h-4 w-4" />
          </button>
          <button
            type="button"
            disabled
            aria-label="New chat"
            className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            <SquarePen className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex flex-1 flex-col items-center justify-center gap-4 overflow-y-auto px-6 py-6 text-center">
        <img src={brandMark} alt="" className="h-12 w-12 opacity-10" />
        <div className="flex flex-wrap justify-center gap-2">
          <button type="button" disabled className={PILL}>
            Summarize issue
          </button>
          <button type="button" disabled className={PILL}>
            Analyze issue
          </button>
        </div>
      </div>

      <div className="border-t border-white/10 bg-black px-4 py-2">
        <p className="truncate text-xs text-slate-400">{ticket.title}</p>
      </div>

      <div className="flex items-center gap-2 border-t border-white/10 p-4">
        <Input disabled placeholder="Ask about this issue…" className="flex-1 text-sm" />
        <button
          type="button"
          disabled
          aria-label="Send message"
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Send className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
