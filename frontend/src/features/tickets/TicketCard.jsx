import { Link } from 'react-router-dom';
import Badge from '../../components/ui/Badge';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from './badgeVariants';

export default function TicketCard({ ticket, groupId }) {
  return (
    <Link
      to={`/tickets/${ticket.id}?group=${groupId}`}
      className="flex items-center justify-between gap-4 rounded-lg border border-white/10 bg-white/5 px-4 py-3 transition-colors hover:bg-white/10"
    >
      <div className="flex min-w-0 items-center gap-3">
        <span className="shrink-0 text-xs font-medium text-slate-500">#{ticket.ticket_number}</span>
        <span className="truncate text-sm font-medium text-white">{ticket.title}</span>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <span className="hidden w-28 truncate text-xs text-slate-400 sm:inline">
          {ticket.created_by_name}
        </span>
        <div className="flex w-20 justify-start">
          <Badge variant={STATUS_VARIANT[ticket.status]}>{capitalize(ticket.status)}</Badge>
        </div>
        <div className="flex w-20 justify-start">
          <Badge variant={PRIORITY_VARIANT[ticket.priority]}>{capitalize(ticket.priority)}</Badge>
        </div>
      </div>
    </Link>
  );
}
