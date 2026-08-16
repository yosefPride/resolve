// Small labeled stat card: icon, uppercase label, big value. Shared by
// GroupStats (per-team detail page) and DashboardStats (cross-team overview),
// so the two don't carry copies of the same markup.
// h-full so a tile fills the row height its grid parent stretches it to,
// rather than sitting shorter than a taller sibling (e.g. one whose label
// wraps to two lines).
export default function StatTile({ icon: Icon, label, value }) {
  return (
    <div className="flex h-full flex-col gap-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3">
      <span className="flex items-center gap-2 text-xs font-medium tracking-wide text-slate-400 uppercase">
        <Icon className="h-4 w-4" />
        {label}
      </span>
      <span className="text-lg font-semibold text-white">{value}</span>
    </div>
  );
}
