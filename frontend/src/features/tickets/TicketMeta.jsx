import Badge from '../../components/ui/Badge';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from './badgeVariants';

// Right-rail summary card for the issue detail page. Timestamps show relative
// ("2 days ago") with the exact datetime on hover via title.
export default function TicketMeta({ ticket }) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-6">
      <dl className="flex flex-col gap-4">
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Priority</dt>
          <dd>
            <Badge size="sm" variant={PRIORITY_VARIANT[ticket.priority]}>
              {capitalize(ticket.priority)}
            </Badge>
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Status</dt>
          <dd>
            <Badge size="sm" variant={STATUS_VARIANT[ticket.status]}>
              {capitalize(ticket.status)}
            </Badge>
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Created</dt>
          <dd className="text-sm text-slate-200" title={formatDateTime(ticket.created_at)}>
            {formatRelativeTime(ticket.created_at)}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4">
          <dt className="text-sm text-slate-500">Updated</dt>
          <dd className="text-sm text-slate-200" title={formatDateTime(ticket.updated_at)}>
            {formatRelativeTime(ticket.updated_at)}
          </dd>
        </div>
      </dl>
    </div>
  );
}
