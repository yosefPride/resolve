// Activity entries stacking onto a timeline, one after another — the landing
// page version of the ticket's activity trail.
const ENTRIES = [
  { width: 'w-28', accent: true },
  { width: 'w-20', accent: false },
  { width: 'w-24', accent: false },
  { width: 'w-16', accent: true },
];

export default function ActivityTrailArt({ active }) {
  return (
    <div className="relative flex h-full w-full flex-col justify-center gap-5 px-2">
      {/* The rail the dots hang off. Inset top and bottom so it reads as a
          slice of a longer history rather than a closed list. */}
      <div className="absolute top-6 bottom-6 left-3 w-px bg-white/10" />

      {ENTRIES.map((entry, index) => (
        <div
          key={index}
          className={`relative flex items-center gap-4 ${active ? 'animate-trail-in' : ''}`}
          style={active ? { animationDelay: `${index * 0.3}s` } : undefined}
        >
          <span
            className={`z-10 size-2 shrink-0 rounded-full ring-4 ring-black ${
              entry.accent ? 'bg-white/70' : 'bg-white/30'
            }`}
          />
          <div className="flex min-w-0 grow flex-col gap-1.5">
            <div className={`h-2 rounded-full bg-white/25 ${entry.width}`} />
            <div className="h-1.5 w-12 rounded-full bg-white/10" />
          </div>
        </div>
      ))}
    </div>
  );
}
