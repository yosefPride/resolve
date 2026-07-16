# Frontend Architecture

---

# Stack

- React
- JavaScript
- Vite
- TailwindCSS

---

# Core Concept: Group-Centric UI

The UI is group-centric, but there is NO "active group" held in state. Instead,
the group is chosen explicitly at the point of use: the Tickets page lists all
the groups the user belongs to, and selecting one loads that group's tickets via
`GET /groups/{id}/tickets` (see design/tickets-design.md). Each request names its
group in the URL, so scope never depends on hidden client state.

- No global system view for regular users
- All data shown belongs to the group whose id is in the request

---

# AI UI Rule

AI features must not exist as a standalone page.

All AI interactions must be embedded inside:

- Ticket Detail page (primary)
- Group Dashboard (reports only)

No separate AI navigation tab or page is allowed.

---

# Application States

## 1. Unauthenticated

- Landing page (public marketing home)
- Login
- Register

## 2. Authenticated, not in any group yet

- Prompt to create a group (shown when the user belongs to none)
- Create group

There is no self-service join: a user only enters a group by being added there by that group's Group Admin (see groups/ below). Once the user belongs to at least one group, the Tickets page lists those groups directly — there is no separate "active group" selection step.

## 3. Active Session

- Full application access
- Logout returns to state 1 (Unauthenticated)

---

# Project Structure

src/

    assets/
    components/
        layout/
        marketing/      (public landing page sections: hero, feature grid, workflow, audience)
        ui/             (design-system primitives: cards, tables, form wrappers)
    features/
        auth/
        account/        (own profile: summary, profile edit, password change)
        groups/
        tickets/
        comments/
        users/
        ai/
        dashboard/
        admin/          (System Admin panels: users, groups, audit log)
    hooks/
    lib/
    pages/
    services/
    utils/
    App.jsx
    main.jsx

---

# Feature Breakdown

## account/

The Account page (`/account`, reached from the user menu) is where a user
manages their own identity — no group or admin scope involved. Three cards:

- Profile summary — read-only header (avatar initials, name, email, member-since,
  a System Admin chip where applicable), rendered straight from the auth context
- Profile edit — update name and/or email (`PATCH /auth/me`). Changing the email
  reveals a required current-password field; a taken email surfaces under the
  Email input via the `duplicate_email` code. On success the auth context user is
  updated in place, so the header/menu reflect a new name without a reload
- Password change — current / new / confirm, with client-side min-length and
  match checks (`POST /auth/me/password`). Succeeds by signing out the user's
  other devices while keeping this session

## groups/

- Create group
- List the groups you belong to (the Tickets page uses this to scope views)
- Manage members (Group Admin only) — added by exact-email lookup (GET /groups/:id/users/lookup), not a browsable directory

## tickets/

- Create ticket
- View tickets (group-scoped)
- Update status (role-dependent)
- Assign tickets (Group Admin only)

Ticket Detail page includes:

- ticket content
- comments
- status/actions
- AI panel (contextual sidebar or section)

## ai/ (contextual feature only)

AI functionality is NOT a standalone page.

AI is embedded inside existing pages:

- Ticket Detail Page (primary location)
  - summarize ticket
  - analyze ticket
  - suggest fixes
  - explain ticket

- Group Dashboard (secondary)
  - AI reports (Group Admin only)

## admin/ (System Admin only)

The Admin page (`/admin`, reached from the user menu) is an additive section
gated by the global System Admin role — it does not replace the regular app, so
a System Admin who is also a group member still uses the normal group UI for
ticket/comment work. The route is guarded by `AdminRoute` (auth + System Admin;
non-admins are redirected to /dashboard); the backend enforces every action via
the `SystemAdminUser` extractor, so this gating is UX-only.

Three tabs, each a panel under `features/admin/`:

- Users — system-wide user list (GET /admin/users). Per-row delete opens the
  succession flow: GET /admin/users/:id/deletion-check classifies the target's
  groups, then the modal collects a successor for each group where they are the
  sole Group Admin (or warns that a sole-member group will be auto-deleted)
  before POST /admin/users/:id/delete. A 409 (the server re-derived a different
  plan because membership shifted) re-runs the check in place. The admin's own
  row has no delete action.
- Groups — system-wide group list (GET /admin/groups, metadata only: name +
  created; no membership, per group isolation). Per-row delete removes the whole
  group (DELETE /admin/groups/:id).
- Audit Log — the succession / auto-deletion trail (GET /admin/audit-log),
  newest-first, filterable by group and by deleted user. Entries carry
  snapshotted names (group, deleted user, successor, performing admin), so even
  a deleted user or auto-deleted group displays its real name; the group/user
  filter options are derived from the log itself.

Consistent with group isolation: the admin sees group *metadata* only, never a
group's tickets, comments, or membership roster — the sole exception being the
eligible-successor list surfaced during a sole-Group-Admin deletion.

---

# RBAC Awareness (Frontend)

Frontend must:

- Hide unavailable actions based on role
- Never rely on UI security (backend is authoritative)
- Always assume server will enforce rules
- reflect user role only for UX purposes

---

# API Layer

- Uses JWT only for identity (the token carries no group)
- Group scope is sent explicitly as the `{id}` path segment on group-scoped
  requests (`/groups/{id}/...`); the backend resolves membership/role from it
- Logout calls POST /auth/logout, then clears the client-stored JWT

## Session Handling

- The access JWT is held in memory only (React context), never in localStorage —
  the refresh token is deliberately httpOnly to keep it out of JS reach, so the
  access token stays equally unexposed to XSS
- On app load, the session is silently restored via POST /auth/refresh followed
  by GET /auth/me, with a loading state shown until this resolves
- A response interceptor transparently refreshes and retries once on a 401 from
  an expired access token mid-session; if the refresh itself fails, the user is
  logged out and redirected to Login

---

# Pages

Public:

- Landing / Home
- Login
- Register

## Setup

- My groups — list the groups you belong to, and create one

## Main App

- Dashboard
- Tickets
- Ticket details (includes AI panel)
- Group management
- Account — own profile: summary, profile edit, password change (see account/ above)
- Admin (System Admin only)
