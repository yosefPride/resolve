import { useState } from 'react';
import { useUpdateTicket } from '../../hooks/useTickets';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';
import Textarea from '../../components/ui/Textarea';

// Group-Admin-only (rendered conditionally by TicketDetail). Status is edited
// here too — there is no separate status endpoint (docs/api.md).
export default function EditTicketForm({ groupId, ticket, onSaved, onCancel }) {
  const [title, setTitle] = useState(ticket.title);
  const [description, setDescription] = useState(ticket.description);
  const [priority, setPriority] = useState(ticket.priority);
  const [status, setStatus] = useState(ticket.status);
  const updateTicket = useUpdateTicket(groupId, ticket.id);
  const { error, submit } = useSubmit(async () => {
    await updateTicket.mutateAsync({ title, description, priority, status });
    onSaved();
  }, 'Failed to save issue.');

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Field label="Title">
        <Input
          type="text"
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          required
          maxLength={200}
        />
      </Field>

      <Field label="Description">
        <Textarea
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          required
          rows={4}
        />
      </Field>

      <Field label="Priority">
        <Select value={priority} onChange={(event) => setPriority(event.target.value)}>
          <option value="low">Low</option>
          <option value="high">High</option>
          <option value="critical">Critical</option>
        </Select>
      </Field>

      <Field label="Status">
        <Select value={status} onChange={(event) => setStatus(event.target.value)}>
          <option value="open">Open</option>
          <option value="closed">Closed</option>
        </Select>
      </Field>

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
