import { useState } from 'react';
import { createGroup } from '../../services/groups.service';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';

export default function CreateGroupForm({ onCreated }) {
  const [name, setName] = useState('');
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    setIsSubmitting(true);
    try {
      const group = await createGroup(name);
      setName('');
      onCreated(group);
    } catch (err) {
      setError(errorMessage(err, 'Failed to create team. Please try again.'));
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Team name
        <Input
          type="text"
          name="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
        />
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={isSubmitting} className="mt-2">
        {isSubmitting ? 'Creating…' : 'Create team'}
      </Button>
    </form>
  );
}
