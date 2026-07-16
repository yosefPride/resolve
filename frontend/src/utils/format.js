// Formats an ISO 8601 / RFC3339 timestamp (what the API returns for created_at,
// joined_at, etc.) as a short human-readable date. Returns '—' for missing or
// unparseable input.
export function formatDate(iso) {
  if (!iso) return '—';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '—';
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
