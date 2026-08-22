import { useState } from 'react';
import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import ConfirmModal from '../../components/ui/ConfirmModal';
import { formatDate } from '../../utils/format';
import { listGroups, deleteGroup } from '../../services/admin.service';
import { errorMessage } from '../../utils/errors';
import { useDebouncedValue } from '../../hooks/useDebouncedValue';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Table, { Td, Tr } from '../../components/ui/Table';

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
          <Table columns={['Name', 'Created', { label: 'Actions', right: true }]}>
            {groups.map((group) => (
              <Tr key={group.id}>
                <Td className="font-medium text-white">{group.name}</Td>
                <Td className="text-slate-400">{formatDate(group.created_at)}</Td>
                <Td className="text-right">
                  <Button variant="dangerOutline" size="sm" onClick={() => setTarget(group)}>
                    Delete team
                  </Button>
                </Td>
              </Tr>
            ))}
          </Table>
        ))}

      <ConfirmModal
        isOpen={!!target}
        onClose={closeModal}
        title="Delete team"
        confirmLabel="Delete team"
        pendingLabel="Deleting…"
        isPending={deleteMutation.isPending}
        error={deleteError}
        onConfirm={handleConfirmDelete}
      >
        Delete <span className="font-semibold text-white">{target?.name}</span> and all of its
        data? This cannot be undone.
      </ConfirmModal>
    </>
  );
}
