import { useState } from 'react';
import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import Modal from '../../components/ui/Modal';
import { formatDate } from '../../utils/format';
import { listGroups, deleteGroup } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';

export default function GroupsPanel() {
  const queryClient = useQueryClient();
  const [target, setTarget] = useState(null); // group pending deletion, or null
  const [deleteError, setDeleteError] = useState('');
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, 300);

  // keepPreviousData: typing swaps results in place instead of flashing a spinner.
  const { data: groups = [], status } = useQuery({
    queryKey: ['admin', 'groups', debouncedSearch],
    queryFn: () => listGroups(debouncedSearch),
    placeholderData: keepPreviousData,
  });

  const deleteMutation = useMutation({
    mutationFn: (id) => deleteGroup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'groups'] });
      setTarget(null);
    },
    onError: (err) => setDeleteError(errorMessage(err, 'Failed to delete team.')),
  });

  function closeModal() {
    if (deleteMutation.isPending) return; // don't abandon an in-flight delete
    setTarget(null);
    setDeleteError('');
  }

  function handleConfirmDelete() {
    setDeleteError('');
    deleteMutation.mutate(target.id);
  }

  return (
    <>
      <Input
        type="search"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder="Search by name"
        aria-label="Search teams"
        className="mb-4 w-full max-w-sm text-sm"
      />

      {status === 'pending' && <p className="text-sm text-slate-400">Loading…</p>}
      {status === 'error' && <p className="text-sm text-red-500">Failed to load teams.</p>}
      {status === 'success' &&
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
              disabled={deleteMutation.isPending}
              className="border border-white/10"
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={handleConfirmDelete}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? 'Deleting…' : 'Delete team'}
            </Button>
          </div>
        </div>
      </Modal>
    </>
  );
}
