// Shared multi-line text field. BASE matches Input's — the two should stay
// visually identical fields, just single- vs multi-line. Pass `className`
// for one-offs (text-sm, heights). All native <textarea> props pass through.

const BASE =
  'rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-sky-400/50 focus:ring-1 focus:ring-sky-400/50';

export default function Textarea({ className = '', ...props }) {
  return <textarea className={`${BASE} ${className}`} {...props} />;
}
