import { Routes, Route } from 'react-router-dom';
import Header from './components/layout/Header';
import Footer from './components/layout/Footer';
import LandingPage from './pages/LandingPage';
import RegisterPage from './pages/RegisterPage';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import AccountPage from './pages/AccountPage';
import MyGroupsPage from './pages/MyGroupsPage';
import GroupManagementPage from './pages/GroupManagementPage';
import AdminPage from './pages/AdminPage';
import ProtectedRoute from './lib/ProtectedRoute';
import AdminRoute from './lib/AdminRoute';

export default function App() {
  return (
    <div className='flex flex-col min-h-screen'> {/* Added min-h-screen for sticky footer */}
      <div className='grow'>
        <Header />
        <Routes>
          <Route path='/' element={<LandingPage />} />
          <Route path='/register' element={<RegisterPage />} />
          <Route path='/login' element={<LoginPage />} />
          <Route
            path='/dashboard'
            element={
              <ProtectedRoute>
                <DashboardPage />
              </ProtectedRoute>
            }
          />
          <Route
            path='/account'
            element={
              <ProtectedRoute>
                <AccountPage />
              </ProtectedRoute>
            }
          />
          <Route
            path='/groups'
            element={
              <ProtectedRoute>
                <MyGroupsPage />
              </ProtectedRoute>
            }
          />
          <Route
            path='/groups/:id'
            element={
              <ProtectedRoute>
                <GroupManagementPage />
              </ProtectedRoute>
            }
          />
          <Route
            path='/admin'
            element={
              <AdminRoute>
                <AdminPage />
              </AdminRoute>
            }
          />
        </Routes>
      </div>
      <Footer />
    </div>
  );
}