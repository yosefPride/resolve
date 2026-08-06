import Badge from '../../components/ui/Badge';

// Generic result bubble for the AI panel — shared by the summary and
// analysis results so styling can't drift between the two. Caller decides
// what goes in `children` (a paragraph for a summary, a field list for an
// analysis); this just handles the loading/error/cached chrome around it.
export default function AiInsightCard({ title, isLoading, error, cached, children }) {
  return (
    <div className="rounded-lg border border-white/10 bg-black/30 p-3">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="text-xs font-semibold text-slate-300">{title}</span>
        {cached && (
          <Badge variant="outline" size="sm">
            Cached
          </Badge>
        )}
      </div>
      {isLoading ? (
        <p className="text-xs text-slate-500">Thinking…</p>
      ) : error ? (
        <p className="text-xs text-red-400">{error}</p>
      ) : (
        <div className="text-xs leading-relaxed text-slate-300">{children}</div>
      )}
    </div>
  );
}
