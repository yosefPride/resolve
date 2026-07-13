// Clipboard/checklist glyph, for the Open Tickets column. Same currentColor
// convention as GroupIcon/RoleIcon/MembersIcon. Checkboxes are left unfilled
// (outline only) to echo "open" rather than "done".
export default function TicketsIcon({ className = 'h-4 w-4' }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true">
      <rect x="5" y="4" width="14" height="17" rx="2" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <rect x="9" y="2.2" width="6" height="3" rx="1" fill="currentColor" />
      <rect x="7.5" y="9" width="2" height="2" fill="none" stroke="currentColor" strokeWidth="1.2" />
      <line x1="11.5" y1="10" x2="16.5" y2="10" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <rect x="7.5" y="13" width="2" height="2" fill="none" stroke="currentColor" strokeWidth="1.2" />
      <line x1="11.5" y1="14" x2="16.5" y2="14" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <rect x="7.5" y="17" width="2" height="2" fill="none" stroke="currentColor" strokeWidth="1.2" />
      <line x1="11.5" y1="18" x2="16.5" y2="18" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}
