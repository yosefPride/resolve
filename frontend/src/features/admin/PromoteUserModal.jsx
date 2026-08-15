import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import Modal from '../../components/ui/Modal';
import { promoteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';

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

  function closeIfIdle() {
    if (promoteMutation.isPending) return;
    onClose();
  }

  return (
    <Modal isOpen onClose={closeIfIdle} title={`Promote ${user.name}`}>
      <div className="flex flex-col gap-4">
        <p className="text-sm text-slate-300">
          Grant <span className="font-semibold text-white">{user.name}</span> ({user.email}) the
          System Admin role? They'll gain access to system-wide user and team metadata. This
          cannot be undone from here.
        </p>

        {submitError && <p className="text-sm text-red-500">{submitError}</p>}

        <div className="flex justify-end gap-2">
          <Button
            variant="ghost"
            onClick={closeIfIdle}
            disabled={promoteMutation.isPending}
            className="border border-white/10"
          >
            Cancel
          </Button>
          <Button onClick={() => promoteMutation.mutate()} disabled={promoteMutation.isPending}>
            {promoteMutation.isPending ? 'Promoting…' : 'Promote to System Admin'}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
