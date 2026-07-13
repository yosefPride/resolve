// Shield-with-user glyph, used for role indicators. Same currentColor
// convention as GroupIcon: shield as a stroked outline, person as a solid
// fill inside it.
export default function RoleIcon({ className = 'h-4 w-4' }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true">
      <path
        d="M12 2.5 L5 5.5 V11 C5 16 8 19.5 12 21 C16 19.5 19 16 19 11 V5.5 Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      <circle cx="12" cy="9.3" r="1.7" fill="currentColor" />
      <path d="M9 15.5 L9 12.3 A3 2 0 0 1 15 12.3 L15 15.5 Z" fill="currentColor" />
    </svg>
  );
}
