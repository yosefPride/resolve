import { useState } from 'react';
import CreateTicketForm from '../tickets/CreateTicketForm';

const SELECT_CLASS =
  'rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

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
        <select
          value={groupId}
          onChange={(event) => setGroupId(event.target.value)}
          className={SELECT_CLASS}
        >
          <option value="" disabled>
            Select a team
          </option>
          {groups.map((group) => (
            <option key={group.id} value={group.id}>
              {group.name}
            </option>
          ))}
        </select>
      </label>

      {groupId && <CreateTicketForm groupId={groupId} onCreated={onCreated} />}
    </div>
  );
}
