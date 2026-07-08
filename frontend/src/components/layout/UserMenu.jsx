import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';

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

  const isSystemAdmin = user?.globalRole === 'SystemAdmin';

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
        <svg viewBox="0 0 24 24" className="h-7 w-7" fill="none" stroke="currentColor" strokeWidth="1.5">
          <circle cx="12" cy="8" r="3.25" />
          <path strokeLinecap="round" d="M5 19c0-3.5 3.13-6 7-6s7 2.5 7 6" />
        </svg>
      </button>

      {isOpen && (
        <div className="absolute right-0 mt-2 w-56 rounded-lg border border-white/10 bg-neutral-950 py-1 shadow-2xl shadow-black/50">
          <div className="flex items-center gap-2 px-4 py-3">
            <span className="relative flex h-2 w-2 shrink-0">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500 opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
            </span>
            <span className="truncate text-sm font-medium text-green-500">{user?.name}</span>
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
            <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 3h4a2 2 0 012 2v14a2 2 0 01-2 2h-4M10 17l5-5-5-5M15 12H3" />
            </svg>
            Log out
          </button>
        </div>
      )}
    </div>
  );
}
