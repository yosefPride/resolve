// Single user silhouette, for the Members column. Same currentColor
// convention as GroupIcon/RoleIcon.
export default function MembersIcon({ className = 'h-4 w-4' }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true">
      <circle cx="12" cy="9" r="3.6" fill="currentColor" />
      <path d="M6 21 L6 14.2 A6 4 0 0 1 18 14.2 L18 21 Z" fill="currentColor" />
    </svg>
  );
}
