import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';

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

      <Select
        value={status}
        onChange={(event) => onStatusChange(event.target.value)}
        aria-label="Filter by status"
      >
        <option value="">All statuses</option>
        <option value="open">Open</option>
        <option value="closed">Closed</option>
      </Select>

      <Select
        value={priority}
        onChange={(event) => onPriorityChange(event.target.value)}
        aria-label="Filter by priority"
      >
        <option value="">All priorities</option>
        <option value="low">Low</option>
        <option value="high">High</option>
        <option value="critical">Critical</option>
      </Select>

      <Select
        value={creator}
        onChange={(event) => onCreatorChange(event.target.value)}
        aria-label="Filter by creator"
      >
        <option value="">Everyone</option>
        {members.map((member) => (
          <option key={member.user_id} value={member.user_id}>
            {member.name}
          </option>
        ))}
      </Select>
    </div>
  );
}
