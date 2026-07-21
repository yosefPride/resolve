import { Link, NavLink } from 'react-router-dom';
import { Bell, LayoutDashboard, Ticket, Users } from 'lucide-react';
import UserMenu from './UserMenu';
import { useAuth } from '../../hooks/useAuth';
import logo from '../../assets/brand-mark.svg';
import Badge from '../ui/Badge';

const NAV_LINKS = [
  { to: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  // Labeled "Issues" in the UI; the route stays /tickets to match the backend.
  { to: '/tickets', label: 'Issues', icon: Ticket },
  { to: '/groups', label: 'Teams', icon: Users },
];

const ROW = 'flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors';

function NavItem({ to, label, icon: Icon }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `${ROW} border-l-2 ${
          isActive
            ? 'border-sky-400 bg-white/10 text-white'
            : 'border-transparent text-slate-400 hover:bg-white/5 hover:text-white'
        }`
      }
    >
      <Icon className="h-4 w-4 shrink-0" />
      {label}
    </NavLink>
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

      <div className="flex items-center gap-3 px-1">
        <UserMenu user={user} onLogout={logout} />
        <span className="truncate text-sm font-medium text-white">{user?.name}</span>
      </div>

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
      </nav>
    </aside>
  );
}
