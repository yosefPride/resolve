import { AlertTriangle, Ticket, User as UserIcon, Users } from 'lucide-react';
import StatTile from '../../ui/StatTile';
import Badge from '../../ui/Badge';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../../../features/tickets/badgeVariants';
import FeatureSpotlight from './FeatureSpotlight';
import { SPOTLIGHT_RECENT, SPOTLIGHT_STATS } from '../demo/spotlightSeed';

const STAT_ICON = {
  Teams: Users,
  'Open Issues': Ticket,
  'Critical/High Open': AlertTriangle,
  'My Open Issues': UserIcon,
};

// A static preview of the real dashboard (features/dashboard/DashboardStats.jsx +
// RecentTickets.jsx): StatTile is reused as-is (pure/presentational); the recent-
// activity rows are recreated with the same Badge/STATUS_VARIANT/PRIORITY_VARIANT
// pattern rather than reusing RecentTickets itself, since that component fetches
// live groups/tickets via React Query.
export default function DashboardSpotlight() {
  return (
    <FeatureSpotlight
      title="Your day, at a glance."
      description="Open issues, what needs attention, and what changed recently — the moment you sign in."
    >
      <div className="rounded-2xl border border-white/10 bg-surface p-6 shadow-2xl shadow-black/50 sm:p-8">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {SPOTLIGHT_STATS.map((stat) => (
            <StatTile key={stat.label} icon={STAT_ICON[stat.label]} label={stat.label} value={stat.value} />
          ))}
        </div>

        <div className="mt-6 flex flex-col gap-3 border-t border-white/10 pt-6">
          <h3 className="text-sm font-semibold text-white">Recent activity</h3>
          <div className="flex flex-col gap-2">
            {SPOTLIGHT_RECENT.map((ticket) => (
              <div
                key={ticket.ticket_number}
                className="flex items-center justify-between gap-4 rounded-lg border border-white/10 bg-white/5 px-4 py-3"
              >
                <div className="flex min-w-0 items-center gap-3">
                  <span className="shrink-0 text-xs font-medium text-slate-500">
                    #{ticket.ticket_number}
                  </span>
                  <span className="truncate text-sm font-medium text-white">{ticket.title}</span>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span className="hidden text-xs text-slate-400 md:inline">{ticket.group_name}</span>
                  <Badge variant={STATUS_VARIANT[ticket.status]}>{capitalize(ticket.status)}</Badge>
                  <Badge variant={PRIORITY_VARIANT[ticket.priority]}>{capitalize(ticket.priority)}</Badge>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </FeatureSpotlight>
  );
}
