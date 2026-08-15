import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import Modal from '../../components/ui/Modal';
import { demoteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';

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

  function closeIfIdle() {
    if (demoteMutation.isPending) return;
    onClose();
  }

  return (
    <Modal isOpen onClose={closeIfIdle} title={`Demote ${user.name}`}>
      <div className="flex flex-col gap-4">
        <p className="text-sm text-slate-300">
          Revoke <span className="font-semibold text-white">{user.name}</span>'s ({user.email})
          System Admin role? They'll lose access to system-wide user and team metadata.
        </p>

        {submitError && <p className="text-sm text-red-500">{submitError}</p>}

        <div className="flex justify-end gap-2">
          <Button
            variant="ghost"
            onClick={closeIfIdle}
            disabled={demoteMutation.isPending}
            className="border border-white/10"
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => demoteMutation.mutate()}
            disabled={demoteMutation.isPending}
          >
            {demoteMutation.isPending ? 'Demoting…' : 'Demote'}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
