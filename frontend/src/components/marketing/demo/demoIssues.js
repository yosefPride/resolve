// Seeded data for the landing-page product demo. Deliberately not wired to the
// API: the landing page renders logged-out (no token, no team scope), and the
// real issues feature is being built on a separate branch. Nothing here imports
// from services/ or features/tickets/ so the two never collide.
//
// Field names and value sets mirror the backend's TicketResponse exactly
// (backend/src/ticket/models.rs) — snake_case, same as every other API payload
// the frontend consumes. Keeping the shape honest means the demo can't drift
// into advertising a workflow the product doesn't have, and swapping in real
// data later would be a straight substitution.

// The backend enums are closed sets, and serde rejects anything outside them
// (see the ticket_status_rejects_unknown_value / ticket_priority_rejects_unknown_value
// tests). There is no 'in_progress' status and no 'medium' priority — do not add
// one here to make the table look busier.
export const DEMO_STATUSES = ['open', 'closed'];
export const DEMO_PRIORITIES = ['low', 'high', 'critical'];

// Resolved once per page load rather than hardcoded, so the demo never shows
// stale dates as the months pass.
const NOW = Date.now();
const HOUR = 60 * 60 * 1000;
const ago = (hours) => new Date(NOW - hours * HOUR).toISOString();

// One fictional team. Names are invented — this renders on a public page, so
// it intentionally avoids real accounts.
export const DEMO_TEAM_NAME = 'Platform';

// The signed-in identity and team list the demo sidebar shows. Invented for the
// same reason as the reporter names above — this renders on a public page.
export const DEMO_USER_NAME = 'Dana Levi';
export const DEMO_TEAMS = [DEMO_TEAM_NAME, 'Mobile', 'Design System'];

const DEMO_GROUP_ID = '6a1f4c9e2b7d8e0f13a45c67';

// ticket_number is a per-team running sequence starting at 1 (allocated by
// TicketRepository::next_ticket_number), not a global id — so a team with six
// issues really is #1 through #6. Ordered newest-first, the way an issue list
// would actually present them.
// Backs the toolbar's person filter. The ticket model has no assignee field —
// only created_by — so this filters by reporter, which is what the mock in
// design/tickets-design.md is actually showing.
export const demoCreators = () => [...new Set(DEMO_ISSUES.map((i) => i.created_by_name))].sort();

export const DEMO_ISSUES = [
  {
    id: '7b2e5d1a4c8f9e0b26d31f48',
    group_id: DEMO_GROUP_ID,
    ticket_number: 6,
    title: 'Refresh token not cleared on logout',
    description:
      'Logging out leaves the refresh cookie in place, so the next visit silently restores the session.',
    status: 'open',
    priority: 'critical',
    created_by: '5c8d2f1e9a3b4c6d7e0f1a2b',
    created_by_name: 'Dana Levi',
    created_at: ago(5),
    updated_at: ago(2),
  },
  {
    id: '3f9c1b7e5d2a8046c1e93b57',
    group_id: DEMO_GROUP_ID,
    ticket_number: 5,
    title: 'Team switcher lists archived teams',
    description:
      'Archived teams still appear in the switcher dropdown and can be selected, landing on an empty workspace.',
    status: 'open',
    priority: 'high',
    created_by: '9e1a7c3d5b2f8046a2c47d19',
    created_by_name: 'Amit Cohen',
    created_at: ago(27),
    updated_at: ago(20),
  },
  {
    id: '1d6b9f2c8e4a7350b9d12e63',
    group_id: DEMO_GROUP_ID,
    ticket_number: 4,
    title: 'Audit log timestamps render in UTC',
    description:
      'Entries ignore the viewer locale, so times read an hour or more off for most of the team.',
    status: 'open',
    priority: 'high',
    created_by: '2b7e4a1c9d5f36089c1a4e72',
    created_by_name: 'Maya Ben-David',
    created_at: ago(52),
    updated_at: ago(49),
  },
  {
    id: '8c3a5e0d7b1f942615e8c3d0',
    group_id: DEMO_GROUP_ID,
    ticket_number: 3,
    title: 'Sidebar collapses on every route change in Safari',
    description:
      'The collapse state resets between navigations on Safari only; Chrome and Firefox persist it correctly.',
    status: 'open',
    priority: 'low',
    created_by: '9e1a7c3d5b2f8046a2c47d19',
    created_by_name: 'Amit Cohen',
    created_at: ago(76),
    updated_at: ago(71),
  },
  {
    id: '4a8d2c6b1e9f507382b6d1a4',
    group_id: DEMO_GROUP_ID,
    ticket_number: 2,
    title: 'Password reset email lands in spam',
    description:
      'Missing SPF alignment on the sending domain pushes reset mail to spam for several providers.',
    status: 'closed',
    priority: 'high',
    created_by: '5c8d2f1e9a3b4c6d7e0f1a2b',
    created_by_name: 'Dana Levi',
    created_at: ago(120),
    updated_at: ago(94),
  },
  {
    id: '6e0b3d9a2c7f8153f2c4a1b8',
    group_id: DEMO_GROUP_ID,
    ticket_number: 1,
    title: 'Table header misaligned below 1280px',
    description:
      'Column headers drift out of alignment with their cells once the viewport drops under 1280px.',
    status: 'closed',
    priority: 'low',
    created_by: '2b7e4a1c9d5f36089c1a4e72',
    created_by_name: 'Maya Ben-David',
    created_at: ago(168),
    updated_at: ago(141),
  },
];
