import { useState } from 'react';
import { ExternalLink, Link as LinkIcon } from 'lucide-react';
import { useAuth } from '../../hooks/useAuth';
import { useCreateReference, useDeleteReference, useReferences } from '../../hooks/useReferences';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';
import CollectionSection from './CollectionSection';

function ReferenceRow({ reference }) {
  return (
    <a
      href={reference.url}
      target="_blank"
      rel="noopener noreferrer"
      className="flex min-w-0 items-center gap-2 text-sm text-sky-300 hover:underline"
    >
      <ExternalLink className="h-4 w-4 shrink-0" />
      <span className="truncate">{reference.label}</span>
    </a>
  );
}

// Rendered inside a Modal (see CollectionSection) — that caps the width.
function AddReferenceForm({ groupId, ticketId, onDone, onCancel }) {
  const [label, setLabel] = useState('');
  const [url, setUrl] = useState('');
  const createReference = useCreateReference(groupId, ticketId);
  const { error, submit } = useSubmit(async () => {
    await createReference.mutateAsync({ label, url });
    onDone();
  }, 'Failed to add reference.');

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Field label="URL">
        <Input
          type="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://…"
          required
        />
      </Field>

      <Field label="Label (optional)">
        <Input
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          placeholder="Defaults to the URL's host"
        />
      </Field>

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
  const query = useReferences(groupId, ticketId);
  const deleteReference = useDeleteReference(groupId, ticketId);

  return (
    <CollectionSection
      icon={LinkIcon}
      title="References"
      addLabel="Add reference"
      addTitle="Add reference"
      loadingText="Loading references…"
      emptyText="No references yet."
      loadErrorFallback="Couldn't load references."
      deleteErrorFallback="Failed to remove reference."
      query={query}
      deleteMutation={deleteReference}
      canDelete={(reference) => reference.created_by === user.id || isAdmin}
      renderRow={(reference) => <ReferenceRow reference={reference} />}
      renderAddForm={(close) => (
        <AddReferenceForm groupId={groupId} ticketId={ticketId} onDone={close} onCancel={close} />
      )}
    />
  );
}
