import { useEffect, useState } from 'react';
import Modal from '../../components/ui/Modal';
import { deletionCheck, deleteUser } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';

// Drives the admin user-deletion flow for a single target user:
//   GET /admin/users/:id/deletion-check  → classify the target's groups
//   POST /admin/users/:id/delete         → commit, resolving succession
// Three branches from the check: plain confirm (no blockers), a required
// successor <select> per sole-admin group, and a warning per auto-delete group.
// A 409 at commit means the server re-derived a different plan (membership
// shifted since the check) — we surface it and re-run the check in place.
export default function DeleteUserModal({ user, onClose, onDeleted }) {
  const [checkStatus, setCheckStatus] = useState('loading'); // loading | ready | error
  const [check, setCheck] = useState(null);
  const [successors, setSuccessors] = useState({}); // group_id -> successor user_id
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState('');

  // Re-fetch and re-classify the target's groups. Used by the retry button and
  // the 409 re-check (both event-driven). Deliberately leaves submitError alone
  // so a 409 message survives the re-check it triggers.
  async function runCheck() {
    try {
      const result = await deletionCheck(user.id);
      setCheck(result);
      // Keep any prior selections that are still valid for the refreshed plan;
      // drop groups that vanished or whose chosen successor is no longer eligible.
      setSuccessors((prev) => {
        const next = {};
        for (const group of result.blocked_groups) {
          const chosen = prev[group.group_id];
          if (chosen && group.eligible_successors.some((m) => m.user_id === chosen)) {
            next[group.group_id] = chosen;
          }
        }
        return next;
      });
      setCheckStatus('ready');
    } catch {
      setCheckStatus('error');
    }
  }

  // Initial check when the modal opens. Inlined (rather than calling runCheck)
  // because the react-hooks lint forbids invoking a setState-ing callback
  // synchronously from an effect; state here is only set in the async callback.
  useEffect(() => {
    let cancelled = false;
    deletionCheck(user.id)
      .then((result) => {
        if (cancelled) return;
        setCheck(result);
        setCheckStatus('ready');
      })
      .catch(() => {
        if (cancelled) return;
        setCheckStatus('error');
      });
    return () => {
      cancelled = true;
    };
  }, [user.id]);

  const blocked = check?.blocked_groups ?? [];
  const autoDelete = check?.auto_delete_groups ?? [];
  const allSuccessorsChosen = blocked.every((group) => successors[group.group_id]);

  async function handleSubmit() {
    setSubmitError('');
    setIsSubmitting(true);
    try {
      await deleteUser(user.id, successors);
      onDeleted();
    } catch (err) {
      if (err.response?.status === 409) {
        setSubmitError(
          errorMessage(err, 'These groups changed since the last check. Please review and try again.'),
        );
        await runCheck();
      } else {
        setSubmitError(errorMessage(err, 'Failed to delete user.'));
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  function closeIfIdle() {
    if (isSubmitting) return;
    onClose();
  }

  return (
    <Modal isOpen onClose={closeIfIdle} title={`Delete ${user.name}`}>
      <div className="flex flex-col gap-4">
        {checkStatus === 'loading' && <p className="text-sm text-slate-400">Checking groups…</p>}

        {checkStatus === 'error' && (
          <>
            <p className="text-sm text-red-500">Couldn't check this user's groups.</p>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={closeIfIdle}
                className="rounded-full border border-white/10 px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  setCheckStatus('loading');
                  runCheck();
                }}
                className="rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-colors hover:bg-white/90"
              >
                Retry
              </button>
            </div>
          </>
        )}

        {checkStatus === 'ready' && (
          <>
            {blocked.length === 0 && autoDelete.length === 0 ? (
              <p className="text-sm text-slate-300">
                Delete <span className="font-semibold text-white">{user.name}</span> ({user.email})?
                This removes their account and every group membership. This cannot be undone.
              </p>
            ) : (
              <p className="text-sm text-slate-300">
                <span className="font-semibold text-white">{user.name}</span> is the sole Group Admin
                of the group(s) below. Resolve each before deleting.
              </p>
            )}

            {blocked.map((group) => (
              <div
                key={group.group_id}
                className="flex flex-col gap-1.5 rounded-lg border border-white/10 bg-white/5 px-4 py-3"
              >
                <p className="text-sm font-medium text-white">{group.group_name}</p>
                <label className="text-xs text-slate-400">
                  Promote a member to Group Admin:
                </label>
                <select
                  value={successors[group.group_id] || ''}
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
                  This group has no other members and will be deleted.
                </p>
              </div>
            ))}

            {submitError && <p className="text-sm text-red-500">{submitError}</p>}

            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={closeIfIdle}
                disabled={isSubmitting}
                className="rounded-full border border-white/10 px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleSubmit}
                disabled={isSubmitting || !allSuccessorsChosen}
                className="rounded-full bg-red-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-500 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isSubmitting ? 'Deleting…' : 'Delete user'}
              </button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
