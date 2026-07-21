import { useState } from 'react';
import { Link, NavLink } from 'react-router-dom';
import {
  Bell,
  ChevronDown,
  CircleUser,
  LayoutDashboard,
  LogOut,
  Shield,
  Ticket,
  User,
  Users,
} from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../../hooks/useAuth';
import { listGroups } from '../../services/groups.service';
import logo from '../../assets/logo.png';
import Badge from '../ui/Badge';

const NAV_LINKS = [
  { to: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  // Labeled "Issues" in the UI; the route stays /tickets to match the backend.
  { to: '/tickets', label: 'Issues', icon: Ticket },
];

const ROW = 'flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors';
const IDLE = 'border-transparent text-slate-400 hover:bg-white/5 hover:text-white';

function NavItem({ to, label, icon: Icon }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `${ROW} border-l-2 ${
          isActive ? 'border-sky-400 bg-white/10 text-white' : IDLE
        }`
      }
    >
      <Icon className="h-4 w-4 shrink-0" />
      {label}
    </NavLink>
  );
}

// The account actions expand inline rather than in a floating menu, so the
// sidebar stays one continuous surface.
function UserSection({ user, onLogout }) {
  const [isOpen, setIsOpen] = useState(false);
  const isSystemAdmin = user?.global_role === 'SystemAdmin';

  return (
    <div className="flex flex-col gap-1">
      <button
        type="button"
        onClick={() => setIsOpen((open) => !open)}
        aria-expanded={isOpen}
        className={`${ROW} w-full border-l-2 ${IDLE}`}
      >
        <CircleUser className="h-5 w-5 shrink-0" strokeWidth={1.5} />
        <span className="truncate text-white">{user?.name}</span>
        <ChevronDown
          className={`ml-auto h-4 w-4 shrink-0 transition-transform duration-200 ${
            isOpen ? 'rotate-180' : ''
          }`}
        />
      </button>

      {isOpen && (
        <div className="flex flex-col gap-1 pl-4">
          <NavItem to="/account" label="Account" icon={User} />
          {isSystemAdmin && <NavItem to="/admin" label="Admin" icon={Shield} />}
          <button
            type="button"
            onClick={onLogout}
            className={`${ROW} w-full border-l-2 ${IDLE}`}
          >
            <LogOut className="h-4 w-4 shrink-0" />
            Log out
          </button>
        </div>
      )}
    </div>
  );
}

// Shares the ['groups'] query key with MyGroupsPage, so creating, renaming or
// deleting a team anywhere in the app refreshes this list through the
// invalidations those pages already run.
function TeamsSection() {
  const [isOpen, setIsOpen] = useState(true);
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center">
        {/* `end` so this row highlights only on the overview — a specific team
            highlights its own row instead. */}
        <NavLink
          to="/groups"
          end
          className={({ isActive }) =>
            `${ROW} grow border-l-2 ${
              isActive ? 'border-sky-400 bg-white/10 text-white' : IDLE
            }`
          }
        >
          <Users className="h-4 w-4 shrink-0" />
          Teams
        </NavLink>
        <button
          type="button"
          onClick={() => setIsOpen((open) => !open)}
          aria-expanded={isOpen}
          aria-label="Toggle team list"
          className="rounded-lg p-2 text-slate-500 transition-colors hover:bg-white/5 hover:text-white"
        >
          <ChevronDown
            className={`h-4 w-4 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`}
          />
        </button>
      </div>

      {isOpen && (
        <div className="flex flex-col gap-1 pl-4">
          {status === 'pending' && <p className="px-3 py-2 text-xs text-slate-500">Loading…</p>}
          {status === 'error' && (
            <p className="px-3 py-2 text-xs text-red-500">Couldn't load teams.</p>
          )}
          {status === 'success' && groups.length === 0 && (
            <p className="px-3 py-2 text-xs text-slate-500">No teams yet.</p>
          )}

          {groups.map((group) => (
            <NavLink
              key={group.id}
              to={`/groups/${group.id}`}
              className={({ isActive }) =>
                `${ROW} border-l-2 ${
                  isActive ? 'border-sky-400 bg-white/10 text-white' : IDLE
                }`
              }
            >
              {({ isActive }) => (
                <>
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      isActive ? 'bg-sky-400' : 'bg-white/20'
                    }`}
                  />
                  <span className="truncate">{group.name}</span>
                </>
              )}
            </NavLink>
          ))}
        </div>
      )}
    </div>
  );
}

export default function Sidebar() {
  const { user, logout } = useAuth();

  return (
    <aside className="sticky top-0 flex h-screen w-64 shrink-0 flex-col gap-4 border-r border-white/10 bg-[#141414] p-3">
      <Link to="/dashboard" className="group flex items-center px-1">
        <img
          src={logo}
          alt="Resolve"
          className="h-12 w-auto object-contain transition-all duration-200 group-hover:drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]"
        />
      </Link>

      <UserSection user={user} onLogout={logout} />

      <nav className="flex flex-col gap-1">
        {/* No notifications backend yet — present but deliberately inert, so the
            row reads as planned rather than broken. */}
        <div
          aria-disabled="true"
          className={`${ROW} cursor-not-allowed border-l-2 border-transparent text-slate-600`}
        >
          <Bell className="h-4 w-4 shrink-0" />
          Notifications
          <Badge variant="outline" size="sm" className="ml-auto">
            soon
          </Badge>
        </div>

        {NAV_LINKS.map((link) => (
          <NavItem key={link.to} {...link} />
        ))}

        <TeamsSection />
      </nav>
    </aside>
  );
}
