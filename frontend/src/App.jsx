import { Routes, Route } from 'react-router-dom';
import MarketingLayout from './components/layout/MarketingLayout';
import AppLayout from './components/layout/AppLayout';
import LandingPage from './pages/LandingPage';
import RegisterPage from './pages/RegisterPage';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import TicketsPage from './pages/TicketsPage';
import TicketDetailPage from './pages/TicketDetailPage';
import AccountPage from './pages/AccountPage';
import GroupManagementPage from './pages/GroupManagementPage';
import AdminPage from './pages/AdminPage';
import NotFoundPage from './pages/NotFoundPage';
import ProtectedRoute from './lib/ProtectedRoute';
import AdminRoute from './lib/AdminRoute';

// Two layout routes: public pages keep the marketing chrome, authenticated
// pages share one AppLayout instance so its chrome never remounts while
// navigating between them. The auth gate wraps the layout (not each page), and
// AdminRoute stays on the /admin leaf since it is an extra role check on top.
//
// NotFoundPage sits outside both groups as its own bare route: it renders
// different chrome depending on auth state (plain vs AppLayout/Sidebar), and
// nesting a second `*` under either group would race the other on route
// ranking instead of reliably covering just that group's visitors.
export default function App() {
  return (
    <Routes>
      <Route element={<MarketingLayout />}>
        <Route path='/' element={<LandingPage />} />
      </Route>

      {/* Bare, no header/footer — a focused auth screen rather than a
          marketing page. */}
      <Route path='/register' element={<RegisterPage />} />
      <Route path='/login' element={<LoginPage />} />

      <Route
        element={
          <ProtectedRoute>
            <AppLayout />
          </ProtectedRoute>
        }
      >
        <Route path='/dashboard' element={<DashboardPage />} />
        <Route path='/tickets' element={<TicketsPage />} />
        <Route path='/tickets/:ticketId' element={<TicketDetailPage />} />
        <Route path='/account' element={<AccountPage />} />
        <Route path='/groups/:id' element={<GroupManagementPage />} />
        <Route
          path='/admin'
          element={
            <AdminRoute>
              <AdminPage />
            </AdminRoute>
          }
        />
      </Route>

      <Route path='*' element={<NotFoundPage />} />
    </Routes>
  );
}
