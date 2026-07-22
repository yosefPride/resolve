import { useState } from 'react';
import { Link, Outlet } from 'react-router-dom';
import { Menu } from 'lucide-react';
import Sidebar from './Sidebar';
import logo from '../../assets/brand-logo.svg';

// Chrome for the authenticated app routes. One Sidebar implementation, rendered
// two ways: docked beside the page from `md` up, and as a slide-in drawer below
// it. Visibility is CSS-driven (`hidden md:flex` / `md:hidden`) so resizing
// never remounts the nav or drops its state.
export default function AppLayout() {
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const closeDrawer = () => setIsDrawerOpen(false);

  return (
    <div className="flex min-h-screen">
      <Sidebar className="sticky top-0 hidden md:flex" />

      {isDrawerOpen && (
        <>
          <button
            type="button"
            onClick={closeDrawer}
            aria-label="Close navigation"
            className="fixed inset-0 z-40 bg-black/70 backdrop-blur-sm md:hidden"
          />
          {/* Not collapsible: a 16-wide rail inside a drawer would be pointless.
              onNavigate closes the drawer once a link is followed. */}
          <Sidebar
            className="fixed inset-y-0 left-0 z-50 flex md:hidden"
            collapsible={false}
            onNavigate={closeDrawer}
          />
        </>
      )}

      <div className="flex min-w-0 grow flex-col">
        <header className="sticky top-0 z-30 flex items-center gap-3 border-b border-white/10 bg-black px-4 py-3 md:hidden">
          {/* Same hamburger the marketing header uses, so the control looks
              identical wherever it appears. */}
          <button
            type="button"
            onClick={() => setIsDrawerOpen(true)}
            aria-label="Open navigation"
            className="flex h-10 w-10 items-center justify-center rounded-lg border border-white/10 bg-white/5 text-slate-300 transition-all duration-200 hover:border-sky-400/50 hover:text-sky-300 hover:ring-2 hover:ring-sky-500/20"
          >
            <Menu className="h-6 w-6" />
          </button>
          <Link to="/dashboard" className="flex items-center">
            <img src={logo} alt="Resolve" className="h-7 w-auto object-contain" />
          </Link>
        </header>

        <main className="grow">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
