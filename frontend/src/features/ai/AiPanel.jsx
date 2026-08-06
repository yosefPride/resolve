import { Maximize2, Send, SquarePen } from 'lucide-react';
import brandMark from '../../assets/brand-mark.svg';
import Input from '../../components/ui/Input';
import { useTicketAnalysis, useTicketSummary } from '../../hooks/useAI';
import { errorMessage } from '../../utils/errors';
import AiInsightCard from './AiInsightCard';

const PILL =
  'rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-60';

// Chatbot-shaped rail for the issue detail page. Summarize/Analyze are wired
// to the AI endpoints (docs/implementation/backend/08-ai.md); chat
// (input/Send/New chat) has no backend endpoint yet and stays disabled.
export default function AiPanel({ ticket, groupId }) {
  const summaryQuery = useTicketSummary(groupId, ticket.id);
  const analysisQuery = useTicketAnalysis(groupId, ticket.id);

  const hasActivity =
    summaryQuery.data ||
    summaryQuery.error ||
    summaryQuery.isFetching ||
    analysisQuery.data ||
    analysisQuery.error ||
    analysisQuery.isFetching;

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
            title="Chat isn't available yet"
            className="flex h-7 w-7 items-center justify-center rounded-full text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            <SquarePen className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex flex-1 flex-col items-center justify-center gap-4 overflow-y-auto px-6 py-6 text-center">
        {!hasActivity && <img src={brandMark} alt="" className="h-12 w-12 opacity-10" />}

        {hasActivity && (
          <div className="flex w-full flex-col gap-3 text-left">
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

      <div className="flex items-center gap-2 border-t border-white/10 p-4">
        <Input
          disabled
          placeholder="Ask about this issue…"
          title="Chat isn't available yet"
          className="flex-1 text-sm"
        />
        <button
          type="button"
          disabled
          aria-label="Send message"
          title="Chat isn't available yet"
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Send className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
