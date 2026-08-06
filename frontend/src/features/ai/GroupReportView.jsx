import { CircleCheck, CircleDot, Clock, Ticket } from 'lucide-react';
import { useGroupReport } from '../../hooks/useAI';
import { errorMessage } from '../../utils/errors';
import { formatRelativeTime } from '../../utils/format';
import { PRIORITY_VARIANT, capitalize } from '../tickets/badgeVariants';
import Badge from '../../components/ui/Badge';
import Button from '../../components/ui/Button';
import StatTile from '../../components/ui/StatTile';

// Group Admin only — the backend rejects a contributor's request with 403
// (docs/implementation/backend/08-ai.md), so GroupManagementPage only mounts
// this when iAmAdmin. Report generation is cheap to re-click: the backend
// caches for 1 hour and returns cached: true within that window.
export default function GroupReportView({ groupId }) {
  const reportQuery = useGroupReport(groupId);
  const report = reportQuery.data;

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-white/10 bg-white/5 p-4">
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="ghost"
          size="sm"
          className="border border-white/10"
          disabled={reportQuery.isFetching}
          onClick={() => reportQuery.refetch()}
        >
          {reportQuery.isFetching ? 'Generating…' : report ? 'Regenerate' : 'Generate report'}
        </Button>
      </div>

      {reportQuery.error && (
        <p className="text-sm text-red-500">
          {errorMessage(reportQuery.error, "Couldn't generate a report.")}
        </p>
      )}

      {report && (
        <>
          <p className="text-sm leading-relaxed text-slate-300">{report.narrative}</p>

          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            <StatTile icon={Ticket} label="Total" value={report.total_tickets} />
            <StatTile icon={CircleDot} label="Open" value={report.open_tickets} />
            <StatTile icon={CircleCheck} label="Closed" value={report.closed_tickets} />
            <StatTile icon={Clock} label="Last 7d" value={report.recent_tickets_7d} />
          </div>

          <div className="flex items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              {Object.entries(report.priority_breakdown).map(([priority, count]) => (
                <Badge key={priority} size="sm" variant={PRIORITY_VARIANT[priority]}>
                  {capitalize(priority)}: {count}
                </Badge>
              ))}
            </div>
            <div className="flex shrink-0 items-center gap-2 text-xs text-slate-500">
              {report.cached && (
                <Badge variant="outline" size="sm">
                  Cached
                </Badge>
              )}
              <span title={report.generated_at}>{formatRelativeTime(report.generated_at)}</span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
