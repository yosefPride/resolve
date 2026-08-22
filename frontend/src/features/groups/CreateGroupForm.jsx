import { useState } from 'react';
import { createGroup } from '../../services/groups.service';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';

export default function CreateGroupForm({ onCreated }) {
  const [name, setName] = useState('');
  const { error, isPending, submit } = useSubmit(async () => {
    const group = await createGroup(name);
    setName('');
    onCreated(group);
  }, 'Failed to create team. Please try again.');

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Field label="Team name">
        <Input
          type="text"
          name="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
        />
      </Field>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={isPending} className="mt-2">
        {isPending ? 'Creating…' : 'Create team'}
      </Button>
    </form>
  );
}
