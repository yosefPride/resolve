// Shared text input. BASE is the field style used across every form and the
// admin search boxes; pass `className` for one-offs (flex-1, widths, and the
// text-sm on the search fields). All native <input> props pass through.

const BASE =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function Input({ className = '', ...props }) {
  return <input className={`${BASE} ${className}`} {...props} />;
}
