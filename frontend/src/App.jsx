import { Routes, Route } from 'react-router-dom';
import MarketingLayout from './components/layout/MarketingLayout';
import AppLayout from './components/layout/AppLayout';
import LandingPage from './pages/LandingPage';
import RegisterPage from './pages/RegisterPage';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import AccountPage from './pages/AccountPage';
import MyGroupsPage from './pages/MyGroupsPage';
import GroupManagementPage from './pages/GroupManagementPage';
import AdminPage from './pages/AdminPage';
import NotFoundPage from './pages/NotFoundPage';
import ProtectedRoute from './lib/ProtectedRoute';
import AdminRoute from './lib/AdminRoute';

// Two layout routes: public pages keep the marketing chrome, authenticated
// pages share one AppLayout instance so its chrome never remounts while
// navigating between them. The auth gate wraps the layout (not each page), and
// AdminRoute stays on the /admin leaf since it is an extra role check on top.
export default function App() {
  return (
    <Routes>
      <Route element={<MarketingLayout />}>
        <Route path='/' element={<LandingPage />} />
        <Route path='/register' element={<RegisterPage />} />
        <Route path='/login' element={<LoginPage />} />
        {/* Catch-all: unmatched paths get the marketing chrome rather than a
            blank page. Sits here (not in AppLayout) so it renders whether or
            not you are signed in. */}
        <Route path='*' element={<NotFoundPage />} />
      </Route>

      <Route
        element={
          <ProtectedRoute>
            <AppLayout />
          </ProtectedRoute>
        }
      >
        <Route path='/dashboard' element={<DashboardPage />} />
        <Route path='/account' element={<AccountPage />} />
        <Route path='/groups' element={<MyGroupsPage />} />
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
    </Routes>
  );
}
