import { Navigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import { isSystemAdmin } from '../utils/roles';
import ProtectedRoute from './ProtectedRoute';

// Composes ProtectedRoute (auth gate → /login) with a System Admin check.
// An authenticated non-admin is bounced to /dashboard. Backend still enforces
// every admin action via SystemAdminUser; this guard is UI-only.
export default function AdminRoute({ children }) {
  const { user } = useAuth();

  return (
    <ProtectedRoute>
      {isSystemAdmin(user) ? children : <Navigate to="/dashboard" replace />}
    </ProtectedRoute>
  );
}
