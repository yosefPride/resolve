import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import ConfirmModal from '../../components/ui/ConfirmModal';
import { demoteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';

// Confirms POST /admin/users/:id/demote — revokes the target's global System
// Admin role. The backend rejects (409) demoting the last remaining System
// Admin; not pre-checked client-side, just surfaced via errorMessage like any
// other failure.
export default function DemoteUserModal({ user, onClose, onDemoted }) {
  const [submitError, setSubmitError] = useState('');

  const demoteMutation = useMutation({
    mutationFn: () => demoteUser(user.id),
    onSuccess: () => onDemoted(),
    onError: (err) => setSubmitError(errorMessage(err, 'Failed to demote user.')),
  });

  return (
    <ConfirmModal
      isOpen
      onClose={onClose}
      title={`Demote ${user.name}`}
      confirmLabel="Demote"
      pendingLabel="Demoting…"
      isPending={demoteMutation.isPending}
      error={submitError}
      onConfirm={() => demoteMutation.mutate()}
    >
      Revoke <span className="font-semibold text-white">{user.name}</span>'s ({user.email})
      System Admin role? They'll lose access to system-wide user and team metadata.
    </ConfirmModal>
  );
}
