import { useMemo, useState } from 'react';
import Input from '../../ui/Input';
import Badge from '../../ui/Badge';
import Pagination from '../../ui/Pagination';
import Select from '../../ui/Select';
import DemoSidebar from './DemoSidebar';
import { PRIORITY_VARIANT, STATUS_VARIANT, capitalize } from '../../../features/tickets/badgeVariants';
import {
  DEMO_ISSUES,
  DEMO_STATUSES,
  DEMO_PRIORITIES,
  DEMO_TEAM_NAME,
  demoCreators,
} from './demoIssues';

// The logged-out product preview that sits in the hero, replacing the static
// screenshot that used to live there. Renders the issues list against seeded
// data (demoIssues.js) — no API call, no auth, no dependency on features/tickets.
//
// Every visual choice here mirrors the real Issues page 1:1 (TicketCard,
// TicketFilters, TicketsPage, badgeVariants.js) so this can't drift into
// looking like a different product than the one behind the login wall. Rows
// stay inert (plain <div>, not <Link>) — clicking through to a real ticket
// page from fake data would either 404 or need auth, either of which reads
// as broken on a public page.

const PER_PAGE = 10;

// Search deliberately only looks at what the row actually shows — title,
// reporter and issue number. Matching on `description` too would be easy, but a
// query that hides five rows and leaves one whose match is nowhere on screen
// reads as a bug rather than a filter.
function matchesQuery(issue, query) {
  if (!query) return true;
  const number = query.replace(/^#/, '');
  return (
    issue.title.toLowerCase().includes(query) ||
    issue.created_by_name.toLowerCase().includes(query) ||
    String(issue.ticket_number) === number
  );
}

export default function ProductDemo() {
  const [query, setQuery] = useState('');
  const [creator, setCreator] = useState('');
  const [status, setStatus] = useState('');
  const [priority, setPriority] = useState('');
  const [page, setPage] = useState(1);

  function clearFilters() {
    setQuery('');
    setCreator('');
    setStatus('');
    setPriority('');
  }

  // Filtering runs against the seeded array in memory. The real issues page
  // pushes this to the API (GET /groups/{id}/tickets with query params);
  // here it stays client-side so the preview works logged-out and offline.
  const issues = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return DEMO_ISSUES.filter(
      (issue) =>
        (!creator || issue.created_by_name === creator) &&
        (!status || issue.status === status) &&
        (!priority || issue.priority === priority) &&
        matchesQuery(issue, normalized),
    );
  }, [query, creator, status, priority]);

  const totalPages = Math.max(1, Math.ceil(issues.length / PER_PAGE));

  return (
    <div className="mx-auto max-h-9xl flex items-stretch overflow-hidden rounded-2xl border border-white/10 bg-surface shadow-2xl shadow-black/50">
      {/* Below `md` the rail would leave the issue list nothing to sit in, so
          the preview narrows to the list alone. */}
      <DemoSidebar className="hidden md:flex" />

      <div className="flex min-w-0 grow flex-col">
        <div className="flex items-center justify-between gap-4 border-b border-white/10 px-5 py-4">
          <h3 className="text-2xl font-bold text-white">{DEMO_TEAM_NAME}</h3>
          <Badge variant="outline" size="sm">
            Preview data
          </Badge>
        </div>

        <div className="flex flex-wrap items-center gap-3 border-b border-white/10 px-5 py-3">
          <Input
            type="search"
            placeholder="Search by title"
            aria-label="Search issues"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            className="flex-1 text-sm sm:max-w-xs"
          />
          <Select
            value={status}
            onChange={(event) => setStatus(event.target.value)}
            aria-label="Filter by status"
          >
            <option value="">All statuses</option>
            {DEMO_STATUSES.map((option) => (
              <option key={option} value={option}>
                {capitalize(option)}
              </option>
            ))}
          </Select>
          <Select
            value={priority}
            onChange={(event) => setPriority(event.target.value)}
            aria-label="Filter by priority"
          >
            <option value="">All priorities</option>
            {DEMO_PRIORITIES.map((option) => (
              <option key={option} value={option}>
                {capitalize(option)}
              </option>
            ))}
          </Select>
          <Select
            value={creator}
            onChange={(event) => setCreator(event.target.value)}
            aria-label="Filter by creator"
          >
            <option value="">Everyone</option>
            {demoCreators().map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </Select>
        </div>

        <div className="flex flex-col gap-4 px-5 py-4">
          {issues.length === 0 && (
            <div className="flex flex-col items-center gap-3 py-6 text-center">
              <p className="text-sm text-slate-400">No issues match those filters.</p>
              <button
                type="button"
                onClick={clearFilters}
                className="rounded-lg border border-white/10 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-white/5 hover:text-white"
              >
                Clear filters
              </button>
            </div>
          )}

          {issues.length > 0 && (
            <div className="flex flex-col gap-2">
              {issues.map((issue) => (
                <div
                  key={issue.id}
                  className="flex items-center justify-between gap-4 rounded-lg border border-white/10 bg-white/5 px-4 py-3"
                >
                  <div className="flex min-w-0 items-center gap-3">
                    <span className="shrink-0 text-xs font-medium text-slate-500">
                      #{issue.ticket_number}
                    </span>
                    <span className="truncate text-sm font-medium text-white">{issue.title}</span>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <span className="hidden text-xs text-slate-400 sm:inline">
                      {issue.created_by_name}
                    </span>
                    <Badge variant={STATUS_VARIANT[issue.status]}>{capitalize(issue.status)}</Badge>
                    <Badge variant={PRIORITY_VARIANT[issue.priority]}>
                      {capitalize(issue.priority)}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          )}

          <div className="flex justify-center">
            <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
          </div>
        </div>
      </div>
    </div>
  );
}
