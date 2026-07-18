import { useState } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import Modal from '../../components/ui/Modal';
import { deletionCheck, deleteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';

// Drives the admin user-deletion flow for a single target user:
//   GET /admin/users/:id/deletion-check  → classify the target's groups
//   POST /admin/users/:id/delete         → commit, resolving succession
// Three branches from the check: plain confirm (no blockers), a required
// successor <select> per sole-admin group, and a warning per auto-delete group.
// A 409 at commit means the server re-derived a different plan (membership
// shifted since the check) — we surface it and re-run the check in place.
export default function DeleteUserModal({ user, onClose, onDeleted }) {
  const [successors, setSuccessors] = useState({}); // group_id -> successor user_id
  const [submitError, setSubmitError] = useState('');

  const deletionQuery = useQuery({
    queryKey: ['admin', 'deletionCheck', user.id],
    queryFn: () => deletionCheck(user.id),
  });
  const check = deletionQuery.data;
  const blocked = check?.blocked_groups ?? [];
  const autoDelete = check?.auto_delete_groups ?? [];

  // The chosen successor for a group, but only if it's still eligible. After a
  // 409 re-check the plan can change, so a prior pick may no longer be valid.
  // Deriving this (instead of pruning successor state in an effect) keeps the
  // enable-state and submit payload correct without syncing state.
  function chosenSuccessor(group) {
    const chosen = successors[group.group_id];
    return group.eligible_successors.some((m) => m.user_id === chosen) ? chosen : '';
  }
  const allSuccessorsChosen = blocked.every((group) => chosenSuccessor(group));

  const deleteMutation = useMutation({
    mutationFn: () => {
      const chosen = {};
      for (const group of blocked) {
        const s = chosenSuccessor(group);
        if (s) chosen[group.group_id] = s;
      }
      return deleteUser(user.id, chosen);
    },
    onSuccess: () => onDeleted(),
    onError: (err) => {
      if (err.response?.status === 409) {
        setSubmitError(
          errorMessage(err, 'These teams changed since the last check. Please review and try again.'),
        );
        deletionQuery.refetch(); // re-run the check in place
      } else {
        setSubmitError(errorMessage(err, 'Failed to delete user.'));
      }
    },
  });

  // 'loading' for the initial check and for a retry-after-error; a background
  // re-check of an already-loaded plan (the 409 path) stays 'ready' so the form
  // doesn't flicker.
  const checkStatus =
    !deletionQuery.isSuccess && deletionQuery.isFetching
      ? 'loading'
      : deletionQuery.isError
        ? 'error'
        : 'ready';

  function handleSubmit() {
    setSubmitError('');
    deleteMutation.mutate();
  }

  function closeIfIdle() {
    if (deleteMutation.isPending) return;
    onClose();
  }

  return (
    <Modal isOpen onClose={closeIfIdle} title={`Delete ${user.name}`}>
      <div className="flex flex-col gap-4">
        {checkStatus === 'loading' && <p className="text-sm text-slate-400">Checking teams…</p>}

        {checkStatus === 'error' && (
          <>
            <p className="text-sm text-red-500">Couldn't check this user's teams.</p>
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={closeIfIdle} className="border border-white/10">
                Cancel
              </Button>
              <Button onClick={() => deletionQuery.refetch()}>Retry</Button>
            </div>
          </>
        )}

        {checkStatus === 'ready' && (
          <>
            {blocked.length === 0 && autoDelete.length === 0 ? (
              <p className="text-sm text-slate-300">
                Delete <span className="font-semibold text-white">{user.name}</span> ({user.email})?
                This removes their account and every team membership. This cannot be undone.
              </p>
            ) : (
              <p className="text-sm text-slate-300">
                <span className="font-semibold text-white">{user.name}</span> is the sole Team Admin
                of the team(s) below. Resolve each before deleting.
              </p>
            )}

            {blocked.map((group) => (
              <div
                key={group.group_id}
                className="flex flex-col gap-1.5 rounded-lg border border-white/10 bg-white/5 px-4 py-3"
              >
                <p className="text-sm font-medium text-white">{group.group_name}</p>
                <label className="text-xs text-slate-400">
                  Promote a member to Team Admin:
                </label>
                <select
                  value={chosenSuccessor(group)}
                  onChange={(event) =>
                    setSuccessors((prev) => ({ ...prev, [group.group_id]: event.target.value }))
                  }
                  className="rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
                >
                  <option value="" disabled>
                    Select a successor…
                  </option>
                  {group.eligible_successors.map((member) => (
                    <option key={member.user_id} value={member.user_id}>
                      {member.name} ({member.email})
                    </option>
                  ))}
                </select>
              </div>
            ))}

            {autoDelete.map((group) => (
              <div
                key={group.group_id}
                className="rounded-lg border border-red-500/30 bg-red-500/5 px-4 py-3"
              >
                <p className="text-sm font-medium text-white">{group.group_name}</p>
                <p className="text-xs text-red-400">
                  This team has no other members and will be deleted.
                </p>
              </div>
            ))}

            {submitError && <p className="text-sm text-red-500">{submitError}</p>}

            <div className="flex justify-end gap-2">
              <Button
                variant="ghost"
                onClick={closeIfIdle}
                disabled={deleteMutation.isPending}
                className="border border-white/10"
              >
                Cancel
              </Button>
              <Button
                variant="danger"
                onClick={handleSubmit}
                disabled={deleteMutation.isPending || !allSuccessorsChosen}
              >
                {deleteMutation.isPending ? 'Deleting…' : 'Delete user'}
              </Button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
