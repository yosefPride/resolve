import { useState } from 'react';
import { Send } from 'lucide-react';
import brandMark from '../../../assets/brand-mark.svg';
import Avatar from '../../ui/Avatar';
import Input from '../../ui/Input';
import AiInsightCard from '../../../features/ai/AiInsightCard';
import FeatureSpotlight from './FeatureSpotlight';
import {
  SPOTLIGHT_ANALYSIS,
  SPOTLIGHT_CHAT_MESSAGES,
  SPOTLIGHT_SUMMARY,
  SPOTLIGHT_TICKET,
} from '../demo/spotlightSeed';

const PILL =
  'rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-60';

// Matches AiPanel.jsx's ChatMessageBubble, which isn't exported from that
// file — copied rather than imported, same reasoning as the rest of this
// preview: nothing here is wired to real conversation data.
function DemoChatBubble({ message }) {
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
          {isAssistant ? 'Assistant' : message.user_name} · {message.relative}
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

// A static preview of the real per-ticket AI panel (features/ai/AiPanel.jsx):
// same chat bubble styling, same Summarize/Analyze pills, same result-card
// layout — AiInsightCard is reused as-is since it's pure/presentational. The
// pills reveal a canned result instead of calling Gemini; the composer is
// decorative, since there's no real ticket behind this preview to send a
// message to.
export default function AiSpotlight() {
  const [showSummary, setShowSummary] = useState(false);
  const [showAnalysis, setShowAnalysis] = useState(true);
  const [draft, setDraft] = useState('');

  return (
    <FeatureSpotlight
      title="Ask about any issue, or get an instant summary and analysis."
      description="Every ticket has its own AI thread — summarize it, analyze it, or just ask a question, without leaving the page."
      tinted
      layout="side"
    >
      <div className="mx-auto flex h-132 max-w-2xl flex-col rounded-xl border border-white/10 bg-white/5 shadow-2xl shadow-black/50">
        <div className="flex items-center justify-between gap-2 border-b border-white/10 px-4 py-3">
          <span className="text-sm font-semibold text-slate-300">New chat</span>
        </div>

        <div className="flex flex-1 flex-col gap-4 overflow-y-auto px-6 py-6">
          <div className="flex w-full flex-col gap-3 text-left">
            {SPOTLIGHT_CHAT_MESSAGES.map((message) => (
              <DemoChatBubble key={message.id} message={message} />
            ))}

            {showSummary && (
              <AiInsightCard title="Summary" cached={false}>
                {SPOTLIGHT_SUMMARY}
              </AiInsightCard>
            )}

            {showAnalysis && (
              <AiInsightCard title="Analysis" cached>
                <dl className="flex flex-col gap-1.5">
                  <div>
                    <dt className="inline font-medium text-slate-400">Severity: </dt>
                    <dd className="inline">{SPOTLIGHT_ANALYSIS.severity_prediction}</dd>
                  </div>
                  <div>
                    <dt className="inline font-medium text-slate-400">Classification: </dt>
                    <dd className="inline">{SPOTLIGHT_ANALYSIS.classification}</dd>
                  </div>
                  <div>
                    <dt className="inline font-medium text-slate-400">Suggested fix: </dt>
                    <dd className="inline">{SPOTLIGHT_ANALYSIS.suggested_fix}</dd>
                  </div>
                </dl>
              </AiInsightCard>
            )}
          </div>

          <div className="flex flex-wrap justify-center gap-2">
            <button type="button" onClick={() => setShowSummary(true)} className={PILL}>
              Summarize issue
            </button>
            <button type="button" onClick={() => setShowAnalysis(true)} className={PILL}>
              Analyze issue
            </button>
          </div>
        </div>

        <div className="border-t border-white/10 bg-black px-4 py-2">
          <p className="truncate text-xs text-slate-400">{SPOTLIGHT_TICKET.title}</p>
        </div>

        <div className="border-t border-white/10 p-4">
          <div className="flex items-center gap-2">
            <Input
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              placeholder="Ask about this issue…"
              aria-label="Ask about this issue"
              className="flex-1 text-sm"
            />
            <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400">
              <Send className="h-4 w-4" />
            </span>
          </div>
        </div>
      </div>
    </FeatureSpotlight>
  );
}
