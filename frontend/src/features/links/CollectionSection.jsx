import { useState } from 'react';
import { Plus } from 'lucide-react';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Modal from '../../components/ui/Modal';

// Shared scaffold for the two Links-tab collections (relation links and
// external references): header with an Add button, the query's
// loading/error/empty/list states, a per-row Remove button with pending
// state, and the add-form modal. Row content and the add form stay with the
// caller — only the choreography lives here.
export default function CollectionSection({
  icon: Icon,
  title,
  addLabel,
  addTitle,
  loadingText,
  emptyText,
  loadErrorFallback,
  deleteErrorFallback,
  query,
  deleteMutation,
  canDelete,
  renderRow,
  renderAddForm,
}) {
  const { data: items, status, error } = query;
  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleteError, setDeleteError] = useState('');
  const [isAdding, setIsAdding] = useState(false);

  async function handleDelete(item) {
    setDeleteError('');
    setPendingDelete(item.id);
    try {
      await deleteMutation.mutateAsync(item.id);
    } catch (err) {
      setDeleteError(errorMessage(err, deleteErrorFallback));
    } finally {
      setPendingDelete(null);
    }
  }

  const closeAdd = () => setIsAdding(false);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-4">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-white">
          <Icon className="h-4 w-4" /> {title}
        </h3>
        <Button
          variant="ghost"
          size="sm"
          className="shrink-0 gap-1.5 border border-white/10"
          onClick={() => setIsAdding(true)}
        >
          <Plus className="h-3.5 w-3.5" /> {addLabel}
        </Button>
      </div>

      {status === 'pending' && <p className="text-sm text-slate-400">{loadingText}</p>}
      {status === 'error' && (
        <p className="text-sm text-red-500">{errorMessage(error, loadErrorFallback)}</p>
      )}
      {status === 'success' &&
        (items.length === 0 ? (
          <p className="text-sm text-slate-500">{emptyText}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {items.map((item) => (
              <li
                key={item.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-white/5 px-4 py-3"
              >
                {renderRow(item)}
                {canDelete(item) && (
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={pendingDelete === item.id}
                    onClick={() => handleDelete(item)}
                    className="shrink-0 text-red-400 hover:text-red-300"
                  >
                    Remove
                  </Button>
                )}
              </li>
            ))}
          </ul>
        ))}
      {deleteError && <p className="text-sm text-red-500">{deleteError}</p>}

      <Modal isOpen={isAdding} onClose={closeAdd} title={addTitle}>
        {renderAddForm(closeAdd)}
      </Modal>
    </div>
  );
}
