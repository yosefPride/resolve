// Badge variant maps for ticket status/priority, shared by TicketCard,
// TicketDetail, and TicketMeta so the colors can't drift apart.

export const PRIORITY_VARIANT = {
  low: 'neutral',
  high: 'accent',
  critical: 'danger',
};

export const STATUS_VARIANT = {
  open: 'accent',
  closed: 'outline',
};

export function capitalize(value) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
