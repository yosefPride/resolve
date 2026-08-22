import { useState } from 'react';
import CreateTicketForm from '../tickets/CreateTicketForm';
import Select from '../../components/ui/Select';

// A dashboard-level "new issue" has no active group to default to
// (docs/frontend.md), so this adds a team picker in front of the existing
// per-team CreateTicketForm — skipped automatically when there's only one
// team to choose from.
export default function NewTicketForm({ groups, onCreated }) {
  const [groupId, setGroupId] = useState(groups.length === 1 ? groups[0].id : '');

  return (
    <div className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm text-slate-300">
        Team
        <Select value={groupId} onChange={(event) => setGroupId(event.target.value)}>
          <option value="" disabled>
            Select a team
          </option>
          {groups.map((group) => (
            <option key={group.id} value={group.id}>
              {group.name}
            </option>
          ))}
        </Select>
      </label>

      {groupId && <CreateTicketForm groupId={groupId} onCreated={onCreated} />}
    </div>
  );
}
