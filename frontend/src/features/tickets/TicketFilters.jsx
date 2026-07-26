import Input from '../../components/ui/Input';

const SELECT_CLASS =
  'rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function TicketFilters({
  search,
  onSearchChange,
  status,
  onStatusChange,
  priority,
  onPriorityChange,
  creator,
  onCreatorChange,
  members,
}) {
  return (
    <div className="flex flex-wrap gap-3">
      <Input
        type="search"
        value={search}
        onChange={(event) => onSearchChange(event.target.value)}
        placeholder="Search by title"
        aria-label="Search issues"
        className="flex-1 text-sm sm:max-w-xs"
      />

      <select
        value={status}
        onChange={(event) => onStatusChange(event.target.value)}
        aria-label="Filter by status"
        className={SELECT_CLASS}
      >
        <option value="">All statuses</option>
        <option value="open">Open</option>
        <option value="closed">Closed</option>
      </select>

      <select
        value={priority}
        onChange={(event) => onPriorityChange(event.target.value)}
        aria-label="Filter by priority"
        className={SELECT_CLASS}
      >
        <option value="">All priorities</option>
        <option value="low">Low</option>
        <option value="high">High</option>
        <option value="critical">Critical</option>
      </select>

      <select
        value={creator}
        onChange={(event) => onCreatorChange(event.target.value)}
        aria-label="Filter by creator"
        className={SELECT_CLASS}
      >
        <option value="">Everyone</option>
        {members.map((member) => (
          <option key={member.user_id} value={member.user_id}>
            {member.name}
          </option>
        ))}
      </select>
    </div>
  );
}
