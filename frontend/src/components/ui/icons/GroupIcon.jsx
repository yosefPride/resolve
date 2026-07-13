// Three-person "team" glyph — a larger figure centered in front, with two
// smaller figures behind on either side, overlapping just enough to read as
// a group. Colored via currentColor so callers control it with Tailwind text
// color utilities, same convention as the icons in Header.jsx/UserMenu.jsx.
export default function GroupIcon({ className = 'h-5 w-5' }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="currentColor" aria-hidden="true">
      <g opacity="0.55">
        <circle cx="5.6" cy="8.4" r="2.5" />
        <path d="M1.5 20 L1.5 13.2 A4.1 2.7 0 0 1 9.7 13.2 L9.7 20 Z" />
      </g>
      <g opacity="0.55">
        <circle cx="18.4" cy="8.4" r="2.5" />
        <path d="M14.3 20 L14.3 13.2 A4.1 2.7 0 0 1 22.5 13.2 L22.5 20 Z" />
      </g>
      <circle cx="12" cy="7" r="3.4" />
      <path d="M6.4 21 L6.4 12.2 A5.6 3.6 0 0 1 17.6 12.2 L17.6 21 Z" />
    </svg>
  );
}
