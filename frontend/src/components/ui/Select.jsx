// Shared dark <select>. BASE is the field style previously copied as a
// SELECT_CLASS constant across the forms, filters, and admin panels; pass
// `className` for one-offs (widths, flex). All native <select> props pass
// through, options go in `children`.
//
// bg-neutral-950 (not Input's bg-white/5) so the dropdown's option list —
// which inherits the element's background in most browsers — stays readable
// instead of rendering translucent.

const BASE =
  'rounded-lg border border-white/10 bg-neutral-950 px-3 py-2 text-sm text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function Select({ className = '', ...props }) {
  return <select className={`${BASE} ${className}`} {...props} />;
}
