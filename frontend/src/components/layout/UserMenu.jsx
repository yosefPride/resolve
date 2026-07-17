import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { User, LogOut } from 'lucide-react';

export default function UserMenu({ user, onLogout }) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef(null);

  useEffect(() => {
    function handleClickOutside(event) {
      if (menuRef.current && !menuRef.current.contains(event.target)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const isSystemAdmin = user?.global_role === 'SystemAdmin';

  return (
    <div className="relative" ref={menuRef}>
      <button
        type="button"
        onClick={() => setIsOpen((open) => !open)}
        title="Open user navigation menu"
        aria-label="Open user navigation menu"
        aria-haspopup="true"
        aria-expanded={isOpen}
        className="flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-white/5 text-slate-300 transition-all duration-200 hover:bg-white/10 hover:ring-1 hover:ring-white focus:ring-1 focus:ring-white focus:scale-95"
      >
        {/* use CircleUser for the sidebar */}
        <User className="h-7 w-7" strokeWidth={1.5} />
      </button>

      {isOpen && (
        <div className="absolute right-0 mt-2 w-56 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50">
          <div className="flex items-center gap-2 px-4 py-3">
            <span className="relative flex h-2 w-2 shrink-0">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-300 opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-green-300" />
            </span>
            <span className="truncate text-sm font-medium text-green-400">{user?.name}</span>
          </div>

          <div className="h-px bg-white/10" />

          <Link
            to="/account"
            onClick={() => setIsOpen(false)}
            className="block px-4 py-2 text-sm text-slate-300 transition-colors hover:bg-white/10"
          >
            Account
          </Link>

          {isSystemAdmin && (
            <Link
              to="/admin"
              onClick={() => setIsOpen(false)}
              className="block px-4 py-2 text-sm text-slate-300 transition-colors hover:bg-white/10"
            >
              Admin
            </Link>
          )}

          <div className="h-px bg-white/10" />

          <button
            type="button"
            onClick={() => {
              setIsOpen(false);
              onLogout();
            }}
            className="flex w-full items-center gap-2 px-4 py-2 text-left text-sm text-slate-300 transition-colors hover:bg-white/10"
          >
            <LogOut className="h-4 w-4" />
            Log out
          </button>
        </div>
      )}
    </div>
  );
}
