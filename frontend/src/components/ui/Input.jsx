import { Search } from 'lucide-react';

// Shared text input. BASE is the field style used across every form and the
// admin search boxes; pass `className` for one-offs (flex-1, widths, and the
// text-sm on the search fields). All native <input> props pass through.
//
// type="search" gets a leading search icon automatically — every search box
// in the app should render one, so it's tied to the type rather than a prop
// callers have to remember to pass.

const BASE =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function Input({ className = '', type, ...props }) {
  if (type === 'search') {
    return (
      <div className={`relative ${className}`}>
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
        <input type="search" className={`${BASE} w-full pl-9`} {...props} />
      </div>
    );
  }

  return <input type={type} className={`${BASE} ${className}`} {...props} />;
}
