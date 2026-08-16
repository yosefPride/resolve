import {
  AlertTriangle,
  FileText,
  Link2,
  MessageSquare,
  Pencil,
  RefreshCw,
  Sparkles,
  Unlink,
} from 'lucide-react';
import { useActivity } from '../../hooks/useActivity';
import { errorMessage } from '../../utils/errors';
import { formatDateTime, formatRelativeTime } from '../../utils/format';
import Badge from '../../components/ui/Badge';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../tickets/badgeVariants';

const EVENT_ICON = {
  ticket_created: Sparkles,
  status_changed: RefreshCw,
  priority_changed: AlertTriangle,
  title_changed: Pencil,
  description_changed: FileText,
  comment_added: MessageSquare,
  comment_deleted: MessageSquare,
  link_added: Link2,
  link_removed: Unlink,
};

// Renders just the verb phrase after the actor's name — "changed status from
// Open to Closed", "added a reference: github.com". link_added/link_removed
// share event types between relation links and external references
// (backend's LinkKind), so link_kind picks the wording here.
function ActivityText({ entry }) {
  switch (entry.event_type) {
    case 'ticket_created':
      return <>created this issue</>;
    case 'status_changed':
      return (
        <>
          changed status from{' '}
          <Badge size="sm" variant={STATUS_VARIANT[entry.old_value]}>
            {capitalize(entry.old_value)}
          </Badge>{' '}
          to{' '}
          <Badge size="sm" variant={STATUS_VARIANT[entry.new_value]}>
            {capitalize(entry.new_value)}
          </Badge>
        </>
      );
    case 'priority_changed':
      return (
        <>
          changed priority from{' '}
          <Badge size="sm" variant={PRIORITY_VARIANT[entry.old_value]}>
            {capitalize(entry.old_value)}
          </Badge>{' '}
          to{' '}
          <Badge size="sm" variant={PRIORITY_VARIANT[entry.new_value]}>
            {capitalize(entry.new_value)}
          </Badge>
        </>
      );
    case 'title_changed':
      return (
        <>
          renamed the issue from &ldquo;{entry.old_value}&rdquo; to &ldquo;{entry.new_value}&rdquo;
        </>
      );
    case 'description_changed':
      return <>updated the description</>;
    case 'comment_added':
      return <>commented</>;
    case 'comment_deleted':
      return <>deleted a comment</>;
    case 'link_added':
      return entry.link_kind === 'reference' ? (
        <>
          added a reference: <span className="text-slate-200">{entry.new_value}</span>
        </>
      ) : (
        <>
          linked this issue: <span className="text-slate-200">{entry.new_value}</span>
        </>
      );
    case 'link_removed':
      return entry.link_kind === 'reference' ? (
        <>
          removed a reference: <span className="text-slate-200">{entry.old_value}</span>
        </>
      ) : (
        <>
          removed a link: <span className="text-slate-200">{entry.old_value}</span>
        </>
      );
    default:
      return null;
  }
}

function ActivityItem({ entry }) {
  const Icon = EVENT_ICON[entry.event_type] ?? Sparkles;
  return (
    <li className="flex gap-3 py-3">
      <div className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-white/10 text-slate-400">
        <Icon className="h-3.5 w-3.5" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-sm text-slate-300">
          <span className="font-medium text-slate-100">{entry.actor_name}</span>{' '}
          <ActivityText entry={entry} />
        </p>
        <p className="mt-0.5 text-xs text-slate-500" title={formatDateTime(entry.occurred_at)}>
          {formatRelativeTime(entry.occurred_at)}
        </p>
      </div>
    </li>
  );
}

export default function ActivityList({ groupId, ticketId }) {
  const { data: entries, status, error } = useActivity(groupId, ticketId);

  if (status === 'pending') {
    return <p className="text-sm text-slate-400">Loading activity…</p>;
  }

  if (status === 'error') {
    return <p className="text-sm text-red-500">{errorMessage(error, "Couldn't load activity.")}</p>;
  }

  if (entries.length === 0) {
    return <p className="text-sm text-slate-500">No activity yet.</p>;
  }

  return (
    <ul className="divide-y divide-white/5">
      {entries.map((entry) => (
        <ActivityItem key={entry.id} entry={entry} />
      ))}
    </ul>
  );
}
