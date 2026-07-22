import { createContext, useContext, useState } from 'react';
import { Link, NavLink } from 'react-router-dom';
import {
  Bell,
  ChevronDown,
  CircleUser,
  LayoutDashboard,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Shield,
  Ticket,
  User,
  Users,
} from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useAuth } from '../../hooks/useAuth';
import { listGroups } from '../../services/groups.service';
import CreateGroupForm from '../../features/groups/CreateGroupForm';
import logo from '../../assets/brand-mark.svg';
import Badge from '../ui/Badge';
import Modal from '../ui/Modal';
import { ROW, rowClasses } from './sidebarStyles';

const NAV_LINKS = [
  { to: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  // Labeled "Issues" in the UI; the route stays /tickets to match the backend.
  { to: '/tickets', label: 'Issues', icon: Ticket },
];

const STORAGE_KEY = 'sidebar:collapsed';

// Collapsed state reaches the nav rows without drilling it through every
// section. `expand` is called on navigation so a click while collapsed both
// follows the link and restores the full sidebar.
const SidebarContext = createContext({
  collapsed: false,
  expand: () => {},
  onNavigate: () => {},
});

function NavItem({ to, label, icon: Icon, end }) {
  const { collapsed, expand, onNavigate } = useContext(SidebarContext);

  function handleClick() {
    expand();
    onNavigate();
  }

  return (
    <NavLink
      to={to}
      end={end}
      onClick={handleClick}
      title={collapsed ? label : undefined}
      className={({ isActive }) => rowClasses(collapsed, isActive)}
    >
      <Icon className="h-4 w-4 shrink-0" />
      {!collapsed && label}
    </NavLink>
  );
}

// The account actions expand inline rather than in a floating menu, so the
// sidebar stays one continuous surface.
function UserSection({ user, onLogout }) {
  const { collapsed, expand } = useContext(SidebarContext);
  const [isOpen, setIsOpen] = useState(false);
  const isSystemAdmin = user?.global_role === 'SystemAdmin';

  // Collapsed, the row is too narrow for the menu, so a click reopens the
  // sidebar and reveals the actions in one step.
  function handleToggle() {
    if (collapsed) {
      expand();
      setIsOpen(true);
      return;
    }
    setIsOpen((open) => !open);
  }

  return (
    <div className="flex flex-col gap-1">
      <button
        type="button"
        onClick={handleToggle}
        aria-expanded={collapsed ? false : isOpen}
        title={collapsed ? user?.name : undefined}
        className={`${rowClasses(collapsed, false)} w-full`}
      >
        <CircleUser className="h-5 w-5 shrink-0" strokeWidth={1.5} />
        {!collapsed && (
          <>
            <span className="truncate text-white">{user?.name}</span>
            <ChevronDown
              className={`ml-auto h-4 w-4 shrink-0 transition-transform duration-200 ${
                isOpen ? 'rotate-180' : ''
              }`}
            />
          </>
        )}
      </button>

      {isOpen && !collapsed && (
        <div className="flex flex-col gap-1 pl-4">
          <NavItem to="/account" label="Account" icon={User} />
          {isSystemAdmin && <NavItem to="/admin" label="Admin" icon={Shield} />}
          <button
            type="button"
            onClick={onLogout}
            className={`${rowClasses(false, false)} w-full`}
          >
            <LogOut className="h-4 w-4 shrink-0" />
            Log out
          </button>
        </div>
      )}
    </div>
  );
}

// Shares the ['groups'] query key with GroupStats, so creating, renaming or
// deleting a team anywhere in the app refreshes this list through the
// invalidations those pages already run.
function TeamsSection() {
  const { collapsed, expand, onNavigate } = useContext(SidebarContext);
  const [isOpen, setIsOpen] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const queryClient = useQueryClient();
  const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });

  function handleCreated() {
    queryClient.invalidateQueries({ queryKey: ['groups'] });
    setIsCreating(false);
    setIsOpen(true); // surface the team that was just created
  }

  // There is no Teams page — the section is a header over the live list, so
  // collapsed it just reopens the sidebar rather than navigating anywhere.
  if (collapsed) {
    return (
      <button
        type="button"
        onClick={expand}
        title="Teams"
        className={`${rowClasses(true, false)} w-full`}
      >
        <Users className="h-4 w-4 shrink-0" />
      </button>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center">
        <button
          type="button"
          onClick={() => setIsOpen((open) => !open)}
          aria-expanded={isOpen}
          className={`${rowClasses(false, false)} grow`}
        >
          <Users className="h-4 w-4 shrink-0" />
          Teams
          <ChevronDown
            className={`ml-auto h-4 w-4 transition-transform duration-200 ${
              isOpen ? 'rotate-180' : ''
            }`}
          />
        </button>
        <button
          type="button"
          onClick={() => setIsCreating(true)}
          title="Create Team"
          aria-label="Create Team"
          className="rounded-lg p-2 text-slate-500 transition-colors hover:bg-white/5 hover:text-white"
        >
          <Plus className="h-4 w-4" />
        </button>
      </div>

      <Modal
        isOpen={isCreating}
        onClose={() => setIsCreating(false)}
        title="Create a team"
      >
        <CreateGroupForm onCreated={handleCreated} />
      </Modal>

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
              onClick={onNavigate}
              className={({ isActive }) => rowClasses(false, isActive)}
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

// `className` carries the positioning/display because Tailwind resolves
// conflicting utilities by CSS source order, not class order — a `fixed` passed
// in would not reliably beat a hardcoded `sticky` here. `collapsible` is off for
// the mobile drawer, where a 16-wide rail would be pointless.
export default function Sidebar({ className = '', collapsible = true, onNavigate }) {
  const { user, logout } = useAuth();
  const [storedCollapsed, setCollapsed] = useState(
    () => localStorage.getItem(STORAGE_KEY) === 'true',
  );
  const collapsed = collapsible && storedCollapsed;

  function updateCollapsed(next) {
    setCollapsed(next);
    localStorage.setItem(STORAGE_KEY, String(next));
  }

  const context = {
    collapsed,
    expand: () => updateCollapsed(false),
    onNavigate: onNavigate ?? (() => {}),
  };

  return (
    <SidebarContext.Provider value={context}>
      <aside
        className={`${className} h-screen shrink-0 flex-col gap-4 border-r border-white/10 bg-black p-3 transition-[width] duration-200 ${
          collapsed ? 'w-16' : 'w-64'
        }`}
      >
        <div
          className={`flex gap-2 ${
            collapsed ? 'flex-col items-center' : 'items-center justify-between px-1'
          }`}
        >
          <Link to="/dashboard" className="group flex items-center">
            <img
              src={logo}
              alt="Resolve"
              className="h-5 w-auto object-contain transition-all duration-200 group-hover:drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]"
            />
          </Link>
          {collapsible && (
            <button
              type="button"
              onClick={() => updateCollapsed(!collapsed)}
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              className="rounded-lg p-1.5 text-slate-500 transition-colors hover:bg-white/5 hover:text-white"
            >
              {collapsed ? (
                <PanelLeftOpen className="h-5 w-5" />
              ) : (
                <PanelLeftClose className="h-5 w-5" />
              )}
            </button>
          )}
        </div>

        <UserSection user={user} onLogout={logout} />

        <nav className="flex flex-col gap-1">
          {/* No notifications backend yet — present but deliberately inert, so
              the row reads as planned rather than broken. */}
          <div
            aria-disabled="true"
            title={collapsed ? 'Notifications (coming soon)' : undefined}
            className={`${ROW} cursor-not-allowed border-l-2 border-transparent text-slate-600 ${
              collapsed ? 'justify-center px-2' : ''
            }`}
          >
            <Bell className="h-4 w-4 shrink-0" />
            {!collapsed && (
              <>
                Notifications
                <Badge variant="outline" size="sm" className="ml-auto">
                  soon
                </Badge>
              </>
            )}
          </div>

          {NAV_LINKS.map((link) => (
            <NavItem key={link.to} {...link} />
          ))}

          <TeamsSection />
        </nav>
      </aside>
    </SidebarContext.Provider>
  );
}
