import { useState } from 'react';
import { useCreateTicket } from '../../hooks/useTickets';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';

const SELECT_CLASS =
  'rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

const TEXTAREA_CLASS =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function CreateTicketForm({ groupId, onCreated }) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState('low');
  const [error, setError] = useState('');
  const createTicket = useCreateTicket(groupId);

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    try {
      const ticket = await createTicket.mutateAsync({ title, description, priority });
      setTitle('');
      setDescription('');
      setPriority('low');
      onCreated(ticket);
    } catch (err) {
      setError(errorMessage(err, 'Failed to create issue.'));
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Title
        <Input
          type="text"
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          required
          maxLength={200}
        />
      </label>

      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Description
        <span className="text-xs text-slate-500">Markdown is supported.</span>
        <textarea
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          required
          rows={4}
          className={TEXTAREA_CLASS}
        />
      </label>

      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Priority
        <select
          value={priority}
          onChange={(event) => setPriority(event.target.value)}
          className={SELECT_CLASS}
        >
          <option value="low">Low</option>
          <option value="high">High</option>
          <option value="critical">Critical</option>
        </select>
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={createTicket.isPending} className="mt-2">
        {createTicket.isPending ? 'Creating…' : 'Create issue'}
      </Button>
    </form>
  );
}
