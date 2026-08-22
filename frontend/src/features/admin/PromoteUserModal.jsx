import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import ConfirmModal from '../../components/ui/ConfirmModal';
import { promoteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';

// Confirms POST /admin/users/:id/promote — grants the target global System
// Admin. No pre-check step like DeleteUserModal's deletion-check: promotion
// has no successor/blocked-group branching, just a single irreversible-in-the-UI
// grant (there is no revoke endpoint yet — see docs/rbac.md).
export default function PromoteUserModal({ user, onClose, onPromoted }) {
  const [submitError, setSubmitError] = useState('');

  const promoteMutation = useMutation({
    mutationFn: () => promoteUser(user.id),
    onSuccess: () => onPromoted(),
    onError: (err) => setSubmitError(errorMessage(err, 'Failed to promote user.')),
  });

  return (
    <ConfirmModal
      isOpen
      onClose={onClose}
      title={`Promote ${user.name}`}
      confirmLabel="Promote to System Admin"
      pendingLabel="Promoting…"
      variant="primary"
      isPending={promoteMutation.isPending}
      error={submitError}
      onConfirm={() => promoteMutation.mutate()}
    >
      Grant <span className="font-semibold text-white">{user.name}</span> ({user.email}) the
      System Admin role? They'll gain access to system-wide user and team metadata. This
      cannot be undone from here.
    </ConfirmModal>
  );
}
