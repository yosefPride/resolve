import {
  Bell,
  ChevronDown,
  CircleUser,
  LayoutDashboard,
  PanelLeftClose,
  Plus,
  Ticket,
  Users,
} from 'lucide-react';
import Badge from '../../ui/Badge';
import logo from '../../../assets/brand-mark.svg';
import { ROW, rowClasses } from '../../layout/sidebarStyles';
import { DEMO_TEAMS, DEMO_TEAM_NAME, DEMO_USER_NAME } from './demoIssues';

// A still of the app sidebar for the landing-page preview. It is not the real
// Sidebar and deliberately so — that component fetches the caller's teams via
// React Query, reads auth, writes the shared `sidebar:collapsed` localStorage
// key, and navigates through NavLink. Rendered logged-out on a public page it
// would fire a 401 for every visitor, hijack the page on click, and stomp the
// signed-in user's collapse preference. Row styling is imported rather than
// copied (layout/sidebarStyles.js) so the two stay visually identical.
//
// Everything here is plain markup: no links, no buttons, no state. The whole
// aside is aria-hidden because it conveys nothing to a screen reader that the
// surrounding marketing copy doesn't already say, and exposing a second, fake
// "Dashboard / Issues" nav on the landing page would be actively confusing.

const NAV = [
  { label: 'Dashboard', icon: LayoutDashboard },
  { label: 'Issues', icon: Ticket, isActive: true },
];

// `className` carries the display utility (e.g. `hidden md:flex`), matching the
// real Sidebar's contract — Tailwind resolves conflicting utilities by source
// order, so display has to come from the caller to win reliably.
export default function DemoSidebar({ className = '' }) {
  return (
    <aside
      aria-hidden="true"
      className={`${className} w-60 shrink-0 flex-col gap-4 border-r border-white/10 bg-black p-3`}
    >
      <div className="flex items-center justify-between gap-2 px-1">
        <img src={logo} alt="" className="h-5 w-auto object-contain" />
        <PanelLeftClose className="h-5 w-5 text-slate-500" />
      </div>

      <div className={`${rowClasses(false, false)} w-full`}>
        <CircleUser className="h-5 w-5 shrink-0" strokeWidth={1.5} />
        <span className="truncate text-white">{DEMO_USER_NAME}</span>
        <ChevronDown className="ml-auto h-4 w-4 shrink-0" />
      </div>

      <div className="flex flex-col gap-1">
        <div className={`${ROW} border-l-2 border-transparent text-slate-600`}>
          <Bell className="h-4 w-4 shrink-0" />
          Notifications
          <Badge variant="outline" size="sm" className="ml-auto">
            soon
          </Badge>
        </div>

        {NAV.map(({ label, icon: Icon, isActive }) => (
          <div key={label} className={rowClasses(false, Boolean(isActive))}>
            <Icon className="h-4 w-4 shrink-0" />
            {label}
          </div>
        ))}

        <div className="flex flex-col gap-1">
          <div className="flex items-center">
            <div className={`${rowClasses(false, false)} grow`}>
              <Users className="h-4 w-4 shrink-0" />
              Teams
              <ChevronDown className="ml-auto h-4 w-4" />
            </div>
            <span className="rounded-lg p-2 text-slate-500">
              <Plus className="h-4 w-4" />
            </span>
          </div>

          <div className="flex flex-col gap-1 pl-4">
            {DEMO_TEAMS.map((team) => {
              const isActive = team === DEMO_TEAM_NAME;
              return (
                <div key={team} className={rowClasses(false, isActive)}>
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      isActive ? 'bg-sky-400' : 'bg-white/20'
                    }`}
                  />
                  <span className="truncate">{team}</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </aside>
  );
}
