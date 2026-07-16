import { formatDate } from '../../utils/format';
import { isSystemAdmin } from '../../utils/roles';

// First letter of the first two words of the name, e.g. "Ada Lovelace" → "AL".
function initials(name) {
  return (name ?? '')
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase() ?? '')
    .join('');
}

// Read-only identity header for the Account page; data comes straight from
// the auth context, so there is nothing to fetch here.
export default function ProfileSummary({ user }) {
  return (
    <div className="flex items-center gap-4 rounded-lg border border-white/10 bg-white/5 p-6">
      <div className="flex h-16 w-16 shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/10 text-xl font-semibold text-white">
        {initials(user?.name)}
      </div>

      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="truncate text-lg font-semibold text-white">{user?.name}</h2>
          {isSystemAdmin(user) && (
            <span className="rounded-full border border-sky-400/30 bg-sky-500/10 px-3 py-1 text-xs font-medium text-sky-300">
              System Admin
            </span>
          )}
        </div>
        <p className="truncate text-sm text-slate-300">{user?.email}</p>
        <p className="mt-1 text-xs text-slate-400">Member since {formatDate(user?.created_at)}</p>
      </div>
    </div>
  );
}
