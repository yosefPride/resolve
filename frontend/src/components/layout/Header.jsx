import { useState } from 'react';
import { Menu } from 'lucide-react';
import { Link, NavLink } from 'react-router-dom';
import UserMenu from './UserMenu';
import { useAuth } from '../../hooks/useAuth';
import logo from '../../assets/logo.png';

const NAV_LINKS = [
  { to: '/dashboard', label: 'Dashboard' },
  { to: '/tickets', label: 'Tickets' },
  { to: '/groups', label: 'Teams' },
];

function NavItem({ to, label, onClick }) {
  return (
    <NavLink
      to={to}
      onClick={onClick}
      className={({ isActive }) =>
        `rounded-full px-3 py-2 text-sm font-medium transition-colors ${
          isActive ? 'bg-white/10 text-white' : 'text-gray-400 hover:bg-white/10 hover:text-white'
        }`
      }
    >
      {label}
    </NavLink>
  );
}

function Logo({ isAuthenticated }) {
  return (
    <Link to={isAuthenticated ? '/dashboard' : '/'} className="group flex items-center">
      <img
        src={logo}
        alt="Resolve"
        className="h-20 w-auto object-contain transition-all duration-200 group-hover:drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]"
      />
    </Link>
  );
}

export default function Header() {
  const { user, status, logout } = useAuth();
  const isAuthenticated = status === 'authenticated';
  const [isMobileNavOpen, setIsMobileNavOpen] = useState(false);

  const handleLogout = () => logout();

  return (
    <header className="sticky top-0 z-50 bg-black/70 backdrop-blur-md">
      <div className="mx-auto flex h-20 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <Logo isAuthenticated={isAuthenticated} />

        <div className="flex items-center gap-6">
          {isAuthenticated && (
            <nav className="hidden items-center gap-3 md:flex">
              {NAV_LINKS.map((link) => (
                <NavItem key={link.to} to={link.to} label={link.label} />
              ))}
            </nav>
          )}

          <div className="flex items-center gap-3">
            {isAuthenticated ? (
              <UserMenu user={user} onLogout={handleLogout} />
            ) : (
              <>
                <Link
                  to="/login"
                  className="rounded-full px-3 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
                >
                  Log in
                </Link>
                <Link
                  to="/register"
                  className="rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-black hover:ring-1 hover:ring-white hover:text-white disabled:cursor-not-allowed disabled:bg-white/50 disabled:text-black/50"
                >
                  Sign up
                </Link>
              </>
            )}

            {isAuthenticated && (
              <button
                type="button"
                onClick={() => setIsMobileNavOpen((open) => !open)}
                aria-label="Toggle navigation"
                className="ml-1 flex h-10 w-10 items-center justify-center rounded-lg border border-white/10 bg-white/5 text-slate-300 transition-all duration-200 hover:border-sky-400/50 hover:text-sky-300 hover:ring-2 hover:ring-sky-500/20 md:hidden"
              >
                <Menu className="h-6 w-6" />
              </button>
            )}
          </div>
        </div>
      </div>

      {isAuthenticated && isMobileNavOpen && (
        <nav className="flex flex-col gap-1 border-t border-white/5 px-4 py-3 md:hidden">
          {NAV_LINKS.map((link) => (
            <NavLink
              key={link.to}
              to={link.to}
              onClick={() => setIsMobileNavOpen(false)}
              className={({ isActive }) =>
                `rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                  isActive ? 'bg-white/10 text-white' : 'text-gray-400 hover:bg-white/10 hover:text-white'
                }`
              }
            >
              {link.label}
            </NavLink>
          ))}
        </nav>
      )}

      <div className="h-px bg-white/10" />
    </header>
  );
}
