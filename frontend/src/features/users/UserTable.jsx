import { formatDate } from '../../utils/format';
import { isSystemAdmin } from '../../utils/roles';

// Presentational: renders the system-wide user list (GET /admin/users →
// UserResponse[]). Loading/error live in the parent panel. The caller's own row
// has no delete action (backend rejects self-deletion anyway).
export default function UserTable({ users, currentUserId, onDelete }) {
  if (users.length === 0) {
    return <p className="text-sm text-slate-400">No users found.</p>;
  }

  return (
    <div className="overflow-x-auto rounded-lg border border-white/10">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
            <th className="px-4 py-3">Name</th>
            <th className="px-4 py-3">Email</th>
            <th className="px-4 py-3">Global Role</th>
            <th className="px-4 py-3">Created</th>
            <th className="px-4 py-3 text-right">Actions</th>
          </tr>
        </thead>
        <tbody>
          {users.map((user) => (
            <tr key={user.id} className="border-b border-white/5 last:border-0 hover:bg-white/5">
              <td className="px-4 py-3 font-medium text-white">{user.name}</td>
              <td className="px-4 py-3 text-slate-300">{user.email}</td>
              <td className="px-4 py-3 text-slate-300">
                {isSystemAdmin(user) ? 'System Admin' : 'User'}
              </td>
              <td className="px-4 py-3 text-slate-400">{formatDate(user.created_at)}</td>
              <td className="px-4 py-3 text-right">
                {user.id === currentUserId ? (
                  <span className="text-xs text-slate-500">You</span>
                ) : (
                  <button
                    type="button"
                    onClick={() => onDelete(user)}
                    className="rounded-full border border-red-500/30 px-3 py-1 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/10"
                  >
                    Delete
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
