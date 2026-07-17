import { useState } from 'react';
import { createGroup } from '../../services/groups.service';
import { errorMessage } from '../../utils/errors';

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
        <input
          type="text"
          name="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
          className="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
        />
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <button
        type="submit"
        disabled={isSubmitting}
        className="mt-2 rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-slate-200 disabled:opacity-50"
      >
        {isSubmitting ? 'Creating…' : 'Create team'}
      </button>
    </form>
  );
}
