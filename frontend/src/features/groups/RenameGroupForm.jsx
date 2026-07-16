import { useState } from 'react';
import { renameGroup } from '../../services/groups.service';
import { errorMessage } from '../../utils/errors';

export default function RenameGroupForm({ groupId, currentName, onRenamed }) {
  const [name, setName] = useState(currentName);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const trimmed = name.trim();
  const unchanged = trimmed === currentName;

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    setIsSubmitting(true);
    try {
      await renameGroup(groupId, trimmed);
      onRenamed();
    } catch (err) {
      setError(errorMessage(err, 'Failed to rename group.'));
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Group name
        <input
          type="text"
          name="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
          autoFocus
          className="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50"
        />
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <button
        type="submit"
        disabled={isSubmitting || trimmed === '' || unchanged}
        className="mt-2 rounded-full bg-white px-4 py-2 text-sm font-semibold text-black transition-all duration-200 hover:bg-slate-200 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {isSubmitting ? 'Saving…' : 'Save'}
      </button>
    </form>
  );
}
