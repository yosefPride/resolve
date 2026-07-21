import { Outlet } from 'react-router-dom';
import Sidebar from './Sidebar';

// Chrome for the authenticated app routes: a persistent left sidebar beside the
// page body. No top header and no footer here — navigation lives in the
// sidebar, and app pages are working surfaces rather than marketing pages.
export default function AppLayout() {
  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <main className="grow">
        <Outlet />
      </main>
    </div>
  );
}
