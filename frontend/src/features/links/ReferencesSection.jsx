import { useState } from 'react';
import { ExternalLink, Link as LinkIcon, Plus } from 'lucide-react';
import { useAuth } from '../../hooks/useAuth';
import { useCreateReference, useDeleteReference, useReferences } from '../../hooks/useReferences';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Modal from '../../components/ui/Modal';

function ReferenceRow({ reference, canDelete, isDeleting, onDelete }) {
  return (
    <li className="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-white/5 px-4 py-3">
      <a
        href={reference.url}
        target="_blank"
        rel="noopener noreferrer"
        className="flex min-w-0 items-center gap-2 text-sm text-sky-300 hover:underline"
      >
        <ExternalLink className="h-4 w-4 shrink-0" />
        <span className="truncate">{reference.label}</span>
      </a>
      {canDelete && (
        <Button
          variant="ghost"
          size="sm"
          disabled={isDeleting}
          onClick={onDelete}
          className="shrink-0 text-red-400 hover:text-red-300"
        >
          Remove
        </Button>
      )}
    </li>
  );
}

// Rendered inside a Modal (see ReferencesSection) — that caps the width.
function AddReferenceForm({ groupId, ticketId, onDone, onCancel }) {
  const [label, setLabel] = useState('');
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  const createReference = useCreateReference(groupId, ticketId);

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    try {
      await createReference.mutateAsync({ label, url });
      onDone();
    } catch (err) {
      setError(errorMessage(err, 'Failed to add reference.'));
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm text-slate-300">
        URL
        <Input
          type="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://…"
          required
        />
      </label>

      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Label (optional)
        <Input
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          placeholder="Defaults to the URL's host"
        />
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <div className="mt-2 flex justify-end gap-3">
        <Button type="button" variant="ghost" disabled={createReference.isPending} onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={!url.trim() || createReference.isPending}>
          {createReference.isPending ? 'Adding…' : 'Add reference'}
        </Button>
      </div>
    </form>
  );
}

export default function ReferencesSection({ groupId, ticketId, isAdmin }) {
  const { user } = useAuth();
  const { data: references, status, error } = useReferences(groupId, ticketId);
  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleteError, setDeleteError] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const deleteReference = useDeleteReference(groupId, ticketId);

  async function handleDelete(reference) {
    setDeleteError('');
    setPendingDelete(reference.id);
    try {
      await deleteReference.mutateAsync(reference.id);
    } catch (err) {
      setDeleteError(errorMessage(err, 'Failed to remove reference.'));
    } finally {
      setPendingDelete(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-4">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-white">
          <LinkIcon className="h-4 w-4" /> References
        </h3>
        <Button
          variant="ghost"
          size="sm"
          className="shrink-0 gap-1.5 border border-white/10"
          onClick={() => setIsAdding(true)}
        >
          <Plus className="h-3.5 w-3.5" /> Add reference
        </Button>
      </div>

      {status === 'pending' && <p className="text-sm text-slate-400">Loading references…</p>}
      {status === 'error' && (
        <p className="text-sm text-red-500">{errorMessage(error, "Couldn't load references.")}</p>
      )}
      {status === 'success' &&
        (references.length === 0 ? (
          <p className="text-sm text-slate-500">No references yet.</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {references.map((reference) => (
              <ReferenceRow
                key={reference.id}
                reference={reference}
                canDelete={reference.created_by === user.id || isAdmin}
                isDeleting={pendingDelete === reference.id}
                onDelete={() => handleDelete(reference)}
              />
            ))}
          </ul>
        ))}
      {deleteError && <p className="text-sm text-red-500">{deleteError}</p>}

      <Modal isOpen={isAdding} onClose={() => setIsAdding(false)} title="Add reference">
        <AddReferenceForm
          groupId={groupId}
          ticketId={ticketId}
          onDone={() => setIsAdding(false)}
          onCancel={() => setIsAdding(false)}
        />
      </Modal>
    </div>
  );
}
