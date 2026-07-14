// Clock glyph, for the Last Activity column. Same currentColor convention as
// the other header icons.
export default function ActivityIcon({ className = 'h-4 w-4' }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" strokeWidth="1.5" />
      <path d="M12 12 L12 7" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M12 12 L15.5 14" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}
