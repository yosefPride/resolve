import { useEffect, useState } from 'react';
import Modal from '../../components/ui/Modal';
import { formatDate } from '../../utils/format';
import { listGroups, deleteGroup } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';

export default function GroupsPanel() {
  const [groups, setGroups] = useState([]);
  const [status, setStatus] = useState('loading');
  const [target, setTarget] = useState(null); // group pending deletion, or null
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');

  useEffect(() => {
    let cancelled = false;
    listGroups()
      .then((data) => {
        if (cancelled) return;
        setGroups(data);
        setStatus('ready');
      })
      .catch(() => {
        if (cancelled) return;
        setStatus('error');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function closeModal() {
    if (isDeleting) return; // don't let a click-away abandon an in-flight delete
    setTarget(null);
    setDeleteError('');
  }

  async function handleConfirmDelete() {
    setDeleteError('');
    setIsDeleting(true);
    try {
      await deleteGroup(target.id);
      // Refetch server truth rather than optimistically splicing (no status
      // flip, so the table doesn't flash a spinner).
      const data = await listGroups();
      setGroups(data);
      setTarget(null);
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete team.'));
    } finally {
      setIsDeleting(false);
    }
  }

  if (status === 'loading') return <p className="text-sm text-slate-400">Loading…</p>;
  if (status === 'error') return <p className="text-sm text-red-500">Failed to load teams.</p>;

  return (
    <>
      {groups.length === 0 ? (
        <p className="text-sm text-slate-400">No teams found.</p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-white/10">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
                <th className="px-4 py-3">Name</th>
                <th className="px-4 py-3">Created</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {groups.map((group) => (
                <tr key={group.id} className="border-b border-white/5 last:border-0 hover:bg-white/5">
                  <td className="px-4 py-3 font-medium text-white">{group.name}</td>
                  <td className="px-4 py-3 text-slate-400">{formatDate(group.created_at)}</td>
                  <td className="px-4 py-3 text-right">
                    <button
                      type="button"
                      onClick={() => setTarget(group)}
                      className="rounded-full border border-red-500/30 px-3 py-1 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/10"
                    >
                      Delete team
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <Modal isOpen={!!target} onClose={closeModal} title="Delete team">
        <div className="flex flex-col gap-4">
          <p className="text-sm text-slate-300">
            Delete <span className="font-semibold text-white">{target?.name}</span> and all of its
            data? This cannot be undone.
          </p>

          {deleteError && <p className="text-sm text-red-500">{deleteError}</p>}

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={closeModal}
              disabled={isDeleting}
              className="rounded-full border border-white/10 px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleConfirmDelete}
              disabled={isDeleting}
              className="rounded-full bg-red-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isDeleting ? 'Deleting…' : 'Delete team'}
            </button>
          </div>
        </div>
      </Modal>
    </>
  );
}
