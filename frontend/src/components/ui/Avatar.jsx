// Initials avatar with a per-user color. The color is picked by hashing
// `seed` (pass a stable id — user_id — so a user keeps their color even if
// renamed; falls back to name) into COLORS, so the same user is always the
// same color without storing anything.

const COLORS = [
  'bg-violet-500/20 text-violet-300',
  'bg-sky-500/20 text-sky-300',
  'bg-green-500/20 text-green-300',
  'bg-amber-500/20 text-amber-300',
  'bg-rose-500/20 text-rose-300',
  'bg-teal-500/20 text-teal-300',
  'bg-orange-500/20 text-orange-300',
  'bg-fuchsia-500/20 text-fuchsia-300',
];

const SIZES = {
  sm: 'h-5 w-5 text-[9px]',
  md: 'h-6 w-6 text-[10px]',
};

function initials(name) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0].toUpperCase())
    .join('');
}

function colorFor(seed) {
  let hash = 0;
  for (const char of seed) {
    hash = (hash * 31 + char.codePointAt(0)) >>> 0;
  }
  return COLORS[hash % COLORS.length];
}

export default function Avatar({ name, seed, size = 'md', className = '' }) {
  const safeName = name || '?';
  return (
    <span
      aria-hidden
      className={`flex shrink-0 items-center justify-center rounded-full font-semibold ${SIZES[size]} ${colorFor(seed || safeName)} ${className}`}
    >
      {initials(safeName)}
    </span>
  );
}
