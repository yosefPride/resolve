import { formatDate } from '../../utils/format';
import { isSystemAdmin } from '../../utils/roles';
import Button from '../../components/ui/Button';
import Table, { Td, Tr } from '../../components/ui/Table';

// Presentational: renders the system-wide user list (GET /admin/users →
// UserResponse[]). Loading/error live in the parent panel. The caller's own
// row has no delete action (backend rejects self-deletion anyway), but does
// get a Demote action — self-demotion is allowed as long as another System
// Admin still exists (backend rejects demoting the last one).
export default function UserTable({ users, currentUserId, onDelete, onPromote, onDemote }) {
  if (users.length === 0) {
    return <p className="text-sm text-slate-400">No users found.</p>;
  }

  return (
    <Table
      columns={['Name', 'Email', 'Global Role', 'Created', { label: 'Actions', right: true }]}
    >
      {users.map((user) => (
        <Tr key={user.id}>
          <Td className="font-medium text-white">{user.name}</Td>
          <Td className="text-slate-300">{user.email}</Td>
          <Td className="text-slate-300">{isSystemAdmin(user) ? 'System Admin' : 'User'}</Td>
          <Td className="text-slate-400">{formatDate(user.created_at)}</Td>
          <Td className="text-right">
            <div className="flex items-center justify-end gap-2">
              {user.id === currentUserId && <span className="text-xs text-slate-500">You</span>}
              {isSystemAdmin(user) ? (
                <Button variant="ghost" size="sm" onClick={() => onDemote(user)}>
                  Demote
                </Button>
              ) : (
                <Button variant="ghost" size="sm" onClick={() => onPromote(user)}>
                  Promote
                </Button>
              )}
              {user.id !== currentUserId && (
                <Button variant="dangerOutline" size="sm" onClick={() => onDelete(user)}>
                  Delete
                </Button>
              )}
            </div>
          </Td>
        </Tr>
      ))}
    </Table>
  );
}
