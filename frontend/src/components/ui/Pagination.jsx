import { ChevronLeft, ChevronRight } from 'lucide-react';

// Which page buttons to show: always 1 and the last page, plus the current
// page and its neighbours. Runs of hidden pages collapse to an ellipsis,
// keyed by the page that follows the gap so keys stay stable.
function pageItems(current, total) {
  const wanted = [1, current - 1, current, current + 1, total]
    .filter((page) => page >= 1 && page <= total)
    .sort((a, b) => a - b);

  const items = [];
  let previous = 0;
  for (const page of wanted) {
    if (page === previous) continue;
    if (page - previous > 1) items.push({ type: 'gap', key: `gap-${page}` });
    items.push({ type: 'page', key: page, page });
    previous = page;
  }
  return items;
}

const CELL = 'flex items-center px-3 py-1.5 transition-colors';

// Single segmented bar: | ‹ Previous | 1 | 2 … 5 | Next › |
// Renders nothing when there's only one page.
export default function Pagination({ page, totalPages, onPageChange }) {
  if (totalPages <= 1) return null;

  return (
    <nav
      aria-label="Pagination"
      className="inline-flex items-stretch divide-x divide-white/10 overflow-hidden rounded-lg border border-white/10 bg-white/5 text-sm"
    >
      <button
        type="button"
        disabled={page <= 1}
        onClick={() => onPageChange(page - 1)}
        className={`${CELL} gap-1 text-slate-400 hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-slate-400`}
      >
        <ChevronLeft className="h-4 w-4" /> Previous
      </button>

      {pageItems(page, totalPages).map((item) =>
        item.type === 'gap' ? (
          <span key={item.key} aria-hidden className={`${CELL} select-none text-slate-500`}>
            …
          </span>
        ) : (
          <button
            key={item.key}
            type="button"
            aria-current={item.page === page ? 'page' : undefined}
            onClick={() => item.page !== page && onPageChange(item.page)}
            className={`${CELL} ${
              item.page === page
                ? 'bg-white/10 font-medium text-white'
                : 'text-slate-400 hover:bg-white/10 hover:text-white'
            }`}
          >
            {item.page}
          </button>
        ),
      )}

      <button
        type="button"
        disabled={page >= totalPages}
        onClick={() => onPageChange(page + 1)}
        className={`${CELL} gap-1 text-slate-400 hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-slate-400`}
      >
        Next <ChevronRight className="h-4 w-4" />
      </button>
    </nav>
  );
}
