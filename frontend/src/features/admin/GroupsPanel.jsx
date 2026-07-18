import { useEffect, useState } from 'react';
import Modal from '../../components/ui/Modal';
import { formatDate } from '../../utils/format';
import { listGroups, deleteGroup } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import Button from '../../components/ui/Button';

export default function GroupsPanel() {
  const [groups, setGroups] = useState([]);
  const [status, setStatus] = useState('loading');
  const [target, setTarget] = useState(null); // group pending deletion, or null
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, 300);

  // Refetches on debounced-term changes. Status only flips to 'loading' for the
  // initial load, so typing swaps results in place (input keeps focus); the
  // cancelled flag drops a response the term has already moved past.
  useEffect(() => {
    let cancelled = false;
    listGroups(debouncedSearch)
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
  }, [debouncedSearch]);

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
      // flip, so the table doesn't flash a spinner), keeping the active filter.
      const data = await listGroups(debouncedSearch);
      setGroups(data);
      setTarget(null);
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to delete team.'));
    } finally {
      setIsDeleting(false);
    }
  }

  return (
    <>
      <input
        type="search"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder="Search by name"
        aria-label="Search teams"
        className="mb-4 w-full max-w-sm rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
      />

      {status === 'loading' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load teams.</p>}
      {status === 'ready' &&
        (groups.length === 0 ? (
          <p className="text-sm text-slate-400">
            {debouncedSearch ? `No teams match “${debouncedSearch}”.` : 'No teams found.'}
          </p>
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
                      <Button variant="dangerOutline" size="sm" onClick={() => setTarget(group)}>
                        Delete team
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}

      <Modal isOpen={!!target} onClose={closeModal} title="Delete team">
        <div className="flex flex-col gap-4">
          <p className="text-sm text-slate-300">
            Delete <span className="font-semibold text-white">{target?.name}</span> and all of its
            data? This cannot be undone.
          </p>

          {deleteError && <p className="text-sm text-red-500">{deleteError}</p>}

          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              onClick={closeModal}
              disabled={isDeleting}
              className="border border-white/10"
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={handleConfirmDelete}
              disabled={isDeleting}
            >
              {isDeleting ? 'Deleting…' : 'Delete team'}
            </Button>
          </div>
        </div>
      </Modal>
    </>
  );
}
