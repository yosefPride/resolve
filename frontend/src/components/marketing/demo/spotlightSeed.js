// Seed data for the landing page's feature spotlights (Issue Detail, AI, and
// Dashboard previews). Kept separate from demoIssues.js — that file backs the
// one filterable Issues-list demo in the hero, this backs three unrelated
// static previews — but reuses the same team/people so the whole page reads
// as one consistent fictional team rather than disconnected demos.

import { DEMO_TEAM_NAME, DEMO_USER_NAME } from './demoIssues';

// Every displayed time on the landing page is a hardcoded string, not a
// Date computed relative to when the page happens to load — these are fixed
// set dressing, not live data, so nothing here should look like it's ticking.

// -- Issue Detail spotlight ---------------------------------------------------

export const SPOTLIGHT_TICKET = {
  ticket_number: 6,
  title: 'Refresh token not cleared on logout',
  description:
    'Logging out leaves the refresh cookie in place, so the next visit silently restores the session.\n\n**Steps to reproduce**\n1. Log in, then log out\n2. Reopen the app without logging in again\n3. The dashboard loads as if still signed in\n\nCookie inspection shows `refresh_token` still set with its original expiry.',
  status: 'open',
  priority: 'critical',
  created_by_name: DEMO_USER_NAME,
  updated_relative: '35 minutes ago',
};

export const SPOTLIGHT_COMMENTS = [
  {
    id: 'c1',
    user_name: 'Amit Cohen',
    user_id: 'demo-amit',
    content: "Confirmed on Safari too — looks like Max-Age isn't being overridden on logout.",
    relative: '2 hours ago',
  },
  {
    id: 'c2',
    user_name: DEMO_USER_NAME,
    user_id: 'demo-dana',
    content: 'Logout handler only cleared the access token. Pushing a fix that expires the refresh cookie too.',
    relative: '35 minutes ago',
  },
];

export const SPOTLIGHT_ACTIVITY = [
  { id: 'a1', event_type: 'ticket_created', actor_name: DEMO_USER_NAME, relative: '5 hours ago' },
  {
    id: 'a2',
    event_type: 'priority_changed',
    actor_name: 'Amit Cohen',
    old_value: 'high',
    new_value: 'critical',
    relative: '3 hours ago',
  },
  { id: 'a3', event_type: 'comment_added', actor_name: DEMO_USER_NAME, relative: '35 minutes ago' },
];

export const SPOTLIGHT_LINK = {
  label: 'blocks',
  other_ticket_number: 9,
  other_ticket_title: 'Session guard redirects to /login in a loop',
  other_ticket_status: 'open',
  other_ticket_priority: 'high',
};

// -- AI spotlight --------------------------------------------------------------

export const SPOTLIGHT_CHAT_MESSAGES = [
  {
    id: 'm1',
    role: 'user',
    user_name: DEMO_USER_NAME,
    user_id: 'demo-dana',
    content: 'What could cause this?',
    relative: '21 minutes ago',
  },
  {
    id: 'm2',
    role: 'assistant',
    content:
      'Most likely the logout handler clears the access-token cookie but not the refresh-token cookie, so the next request silently mints a new session from it. Check that both cookies share the same clear-cookie call.',
    relative: '20 minutes ago',
  },
];

export const SPOTLIGHT_SUMMARY =
  'Logging out clears the access token but leaves the refresh cookie in place, letting the next visit silently restore the session.';

export const SPOTLIGHT_ANALYSIS = {
  severity_prediction: 'Critical',
  classification: 'Security / Auth',
  suggested_fix: 'Clear the refresh-token cookie in the same logout handler that clears the access token.',
};

// -- Dashboard spotlight ---------------------------------------------------------

export const SPOTLIGHT_STATS = [
  { label: 'Teams', value: 3 },
  { label: 'Open Issues', value: 14 },
  { label: 'Critical/High Open', value: 4 },
  { label: 'My Open Issues', value: 5 },
];

export const SPOTLIGHT_RECENT = [
  { ticket_number: 6, title: 'Refresh token not cleared on logout', group_name: DEMO_TEAM_NAME, status: 'open', priority: 'critical' },
  { ticket_number: 5, title: 'Team switcher lists archived teams', group_name: DEMO_TEAM_NAME, status: 'open', priority: 'high' },
  { ticket_number: 12, title: 'Export button missing on Mobile', group_name: 'Mobile', status: 'open', priority: 'low' },
  { ticket_number: 2, title: 'Password reset email lands in spam', group_name: DEMO_TEAM_NAME, status: 'closed', priority: 'high' },
];
