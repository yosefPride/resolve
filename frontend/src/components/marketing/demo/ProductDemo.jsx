import { Search } from 'lucide-react';
import Input from '../../ui/Input';
import Badge from '../../ui/Badge';
import DemoSidebar from './DemoSidebar';
import { formatDate } from '../../../utils/format';
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
// Rows are intentionally inert: opening an issue needs a detail page that is
// being built elsewhere, so there is nothing to navigate to. The toolbar filters
// become live in the next stage; they are rendered here unstyled-as-disabled on
// purpose, since greyed-out controls on a landing page read as broken.

// Shared column template so the header and every row stay aligned. The trailing
// date column is dropped below `lg` where there isn't room for it.
const COLUMNS =
  'grid grid-cols-[minmax(0,1fr)_7rem_5.5rem] gap-4 lg:grid-cols-[minmax(0,1fr)_9rem_6rem_6rem_6rem]';

// Badge only ships neutral/accent/outline — no semantic colours — so the status
// and priority pills are local to the demo rather than new shared variants.
// Worth promoting into Badge if the real issues page wants the same treatment.
const STATUS_STYLES = {
  open: 'bg-emerald-500/10 text-emerald-300 border border-emerald-400/20',
  closed: 'bg-white/5 text-slate-400 border border-white/10',
};

const PRIORITY_STYLES = {
  critical: 'bg-red-500/10 text-red-300 border border-red-400/20',
  high: 'bg-amber-500/10 text-amber-300 border border-amber-400/20',
  low: 'bg-white/5 text-slate-400 border border-white/10',
};

function Pill({ styles, children }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium capitalize ${styles}`}
    >
      {children}
    </span>
  );
}

// Native <select> rather than the app's Radix dropdowns: it is keyboard- and
// screen-reader-accessible for free, and the demo has no need for the styled
// menu behaviour. Swap to DropdownMenu if the preview needs to match the real
// toolbar pixel for pixel.
function FilterSelect({ label, options, ...props }) {
  return (
    <label className="flex items-center gap-2">
      <span className="sr-only">{label}</span>
      <select
        className="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-300 capitalize outline-none focus:border-sky-400/50"
        {...props}
      >
        <option value="">{label}</option>
        {options.map((option) => (
          <option
            key={option}
            value={option}
            className="bg-slate-900 capitalize"
          >
            {option}
          </option>
        ))}
      </select>
    </label>
  );
}

export default function ProductDemo() {
  const issues = DEMO_ISSUES;

  return (
    <div className="mx-auto flex items-stretch overflow-hidden rounded-2xl border border-white/10 bg-[#141414] shadow-2xl shadow-black/50">
      {/* Below `md` the rail would leave the issue list nothing to sit in, so
          the preview narrows to the list alone. */}
      <DemoSidebar className="hidden md:flex" />

      <div className="flex min-w-0 grow flex-col">
        <div className="flex items-center justify-between gap-4 border-b border-white/10 px-5 py-4">
          <div className="flex items-baseline gap-3">
            <h3 className="text-sm font-semibold text-white">
              {DEMO_TEAM_NAME}
            </h3>
            <span className="text-xs text-slate-400">
              {issues.filter((issue) => issue.status === 'open').length} open
            </span>
          </div>
          <Badge variant="outline" size="sm">
            Sample data
          </Badge>
        </div>

        <div className="flex flex-wrap items-center gap-3 border-b border-white/10 px-5 py-3">
          <div className="relative flex-1 basis-48">
            <Search className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-slate-500" />
            <Input
              type="search"
              placeholder="Search issues"
              aria-label="Search issues"
              className="w-full py-1.5 pl-9 text-sm"
            />
          </div>
          <FilterSelect label="Reporter" options={demoCreators()} />
          <FilterSelect label="Status" options={DEMO_STATUSES} />
          <FilterSelect label="Priority" options={DEMO_PRIORITIES} />
        </div>

        <div
          className={`${COLUMNS} border-b border-white/10 px-5 py-2 text-xs font-medium tracking-wide text-slate-500 uppercase`}
        >
          <span>Issue</span>
          <span className="hidden lg:block">Reporter</span>
          <span>Status</span>
          <span>Priority</span>
          <span className="hidden lg:block">Created</span>
        </div>

        <ul className="divide-y divide-white/5">
          {issues.map((issue) => (
            <li key={issue.id} className={`${COLUMNS} items-center px-5 py-3`}>
              <div className="flex min-w-0 items-baseline gap-2">
                <span className="text-xs text-slate-500 tabular-nums">
                  #{issue.ticket_number}
                </span>
                <span className="truncate text-sm text-white">
                  {issue.title}
                </span>
              </div>
              <span className="hidden truncate text-sm text-slate-400 lg:block">
                {issue.created_by_name}
              </span>
              <Pill styles={STATUS_STYLES[issue.status]}>{issue.status}</Pill>
              <Pill styles={PRIORITY_STYLES[issue.priority]}>
                {issue.priority}
              </Pill>
              <span className="hidden text-xs text-slate-500 lg:block">
                {formatDate(issue.created_at)}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
