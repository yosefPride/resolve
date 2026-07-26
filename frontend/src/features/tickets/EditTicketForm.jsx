import { useState } from 'react';
import { useUpdateTicket } from '../../hooks/useTickets';
import { errorMessage } from '../../utils/errors';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';

const SELECT_CLASS =
  'rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

const TEXTAREA_CLASS =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

// Group-Admin-only (rendered conditionally by TicketDetail). Status is edited
// here too — there is no separate status endpoint (docs/api.md).
export default function EditTicketForm({ groupId, ticket, onSaved, onCancel }) {
  const [title, setTitle] = useState(ticket.title);
  const [description, setDescription] = useState(ticket.description);
  const [priority, setPriority] = useState(ticket.priority);
  const [status, setStatus] = useState(ticket.status);
  const [error, setError] = useState('');
  const updateTicket = useUpdateTicket(groupId, ticket.id);

  async function handleSubmit(event) {
    event.preventDefault();
    setError('');
    try {
      await updateTicket.mutateAsync({ title, description, priority, status });
      onSaved();
    } catch (err) {
      setError(errorMessage(err, 'Failed to save issue.'));
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

      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Status
        <select
          value={status}
          onChange={(event) => setStatus(event.target.value)}
          className={SELECT_CLASS}
        >
          <option value="open">Open</option>
          <option value="closed">Closed</option>
        </select>
      </label>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <div className="mt-2 flex justify-end gap-3">
        <Button type="button" variant="ghost" disabled={updateTicket.isPending} onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={updateTicket.isPending}>
          {updateTicket.isPending ? 'Saving…' : 'Save changes'}
        </Button>
      </div>
    </form>
  );
}
