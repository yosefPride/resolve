import { useState } from 'react';
import { useCreateTicket } from '../../hooks/useTickets';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';
import Textarea from '../../components/ui/Textarea';

export default function CreateTicketForm({ groupId, onCreated }) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState('low');
  const createTicket = useCreateTicket(groupId);
  const { error, submit } = useSubmit(async () => {
    const ticket = await createTicket.mutateAsync({ title, description, priority });
    setTitle('');
    setDescription('');
    setPriority('low');
    onCreated(ticket);
  }, 'Failed to create issue.');

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
        <span className="text-xs text-slate-500">Markdown is supported.</span>
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

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={createTicket.isPending} className="mt-2">
        {createTicket.isPending ? 'Creating…' : 'Create issue'}
      </Button>
    </form>
  );
}
