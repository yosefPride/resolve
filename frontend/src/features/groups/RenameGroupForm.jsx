import { useState } from 'react';
import { renameGroup } from '../../services/groups.service';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';

export default function RenameGroupForm({ groupId, currentName, onRenamed }) {
  const [name, setName] = useState(currentName);

  const trimmed = name.trim();
  const unchanged = trimmed === currentName;

  const { error, isPending, submit } = useSubmit(async () => {
    await renameGroup(groupId, trimmed);
    onRenamed();
  }, 'Failed to rename team.');

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Field label="Team name">
        <Input
          type="text"
          name="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
          autoFocus
        />
      </Field>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={isPending || trimmed === '' || unchanged} className="mt-2">
        {isPending ? 'Saving…' : 'Save'}
      </Button>
    </form>
  );
}
