# Admin Page — Staged Build Plan

Branch: `feature/frontend-admin`. Additive admin model: same app shell, an extra
**Admin** nav link visible only when `isSystemAdmin(user)` (already in
`frontend/src/utils/roles.js`). Backend enforces everything via `SystemAdminUser`;
the frontend is UI-only.

Existing backend surface (all guarded, all tested):

- `GET /admin/users`
- `GET /admin/groups`
- `DELETE /admin/groups/{id}`
- `GET /admin/users/{id}/deletion-check` → `blocked_groups[]` (with
  `eligible_successors`), `auto_delete_groups[]`
- `POST /admin/users/{id}/delete` with `successors: { group_id: user_id }`,
  409 on stale/missing successor

Missing backend piece: audit-log **read** endpoint (repo methods
`list_audit_log_for_group` / `list_audit_log_for_user` exist, unused) and the
deferred `admin_audit_log` indexes.

User commits per stage. Every frontend stage ends with a manual browser
checklist handed to the user (they run browser verification themselves).

---

## Stage 1 — Backend: audit-log read endpoint + deferred indexes

- `ensure_indexes` (backend/src/db.rs): add `admin_audit_log` indexes on
  `group_id` and `deleted_user_id`.
- `GET /admin/audit-log` with optional `?group_id=` / `?user_id=` query filters
  (user_id = the deleted user). `SystemAdminUser` guard, same pattern as the
  other admin handlers. New response model serializing ObjectIds as strings.
  No filter = both filters absent is allowed (returns recent entries) or we
  require one filter — decide at implementation; default: allow unfiltered,
  sorted newest-first.
- Docs: api.md (endpoint), database.md (indexes no longer "deferred"),
  backend.md if it lists admin endpoints.

**Tests** (`--test-threads=1`; never exact counts in `*_api_tests.rs` —
shared collections):

- `admin_service_tests.rs`: non-admin caller rejected; group filter returns
  only that group's entries; user filter likewise.
- `admin_api_tests.rs`: 401 without token; 403 for regular user; perform a
  succession-producing user deletion, then assert the specific new entry
  appears in `GET /admin/audit-log?group_id=...` (match by ids, not count).

## Stage 2 — Frontend foundations: test runner + admin service + guarded route

- **Introduce Vitest + React Testing Library + jsdom** (first frontend test
  infra in the repo; `npm test` script). One-time cost paid here, every later
  stage reuses it.
- `frontend/src/services/admin.service.js` (users.service.js stays for
  non-admin user ops): `listUsers`, `listGroups`, `deleteGroup`,
  `deletionCheck(userId)`, `deleteUser(userId, successors)`,
  `listAuditLog(filters)`.
- `AdminRoute` guard (composes `ProtectedRoute` + `isSystemAdmin`, redirects
  non-admins to `/dashboard`); `/admin` route in App.jsx; **Admin** nav link in
  the header/user menu rendered only for system admins.
- `AdminPage.jsx`: skeleton with section tabs — Users / Groups / Audit Log.

**Tests:** service functions hit the right URL/method/payload (mocked axios);
`AdminRoute` redirects unauthenticated and non-admin users, renders children
for admin; nav link hidden for regular user, shown for admin.
**Manual checklist:** admin sees link + empty page; contributor doesn't see
link and gets redirected from `/admin`.

## Stage 3 — Read-only sections: user list, group list, group deletion

- Users tab: fill `features/users/UserTable.jsx` (0-byte stub) — username,
  email, global role, created; loading/empty/error states.
- Groups tab: metadata-only table (name, member count, created — whatever
  `GroupResponse` exposes); per-row **Delete group** with confirm modal
  (reuse `components/ui/Modal.jsx`), refresh list on success.

**Tests:** tables render mocked data; loading and error states; delete flow —
confirm calls `deleteGroup` with right id, cancel doesn't, list refetches.
**Manual checklist:** real data renders; group deletion works end-to-end.

## Stage 4 — User deletion with succession flow

The core admin feature. Delete button per user row →
`GET deletion-check` → modal with three branches:

- no blockers, no auto-deletes → plain confirm;
- `blocked_groups[]` → one required successor `<select>` per group, populated
  from `eligible_successors`; submit disabled until every group has a choice;
- `auto_delete_groups[]` → listed with an explicit "this group will be
  deleted" warning.

Submit → `POST .../delete` with the `successors` map. On 409 (stale check),
show message and re-run deletion-check into the same modal. Self-deletion:
hide/disable delete on the caller's own row (backend rejects anyway).

**Tests** (heaviest stage): each modal branch renders correctly from mocked
check responses; submit gating (disabled until all successors picked); exact
`successors` payload shape; 409 → error shown + check re-fired; success →
modal closes + user list refetches.
**Manual checklist:** run the three real scenarios (plain user, sole-admin
user with successor pick, only-member user auto-delete) and verify audit
entries land.

## Stage 5 — Audit-log viewer

- Audit Log tab: filter controls (by group / by deleted user — populated from
  the already-loaded lists), entries table: action, group, deleted user,
  successor (if any), performed by, timestamp (`utils/format.js`).
- docs/frontend.md updated to describe the finished admin page.

**Tests:** filters produce correct query params; entries render; empty state;
filter switch triggers refetch.
**Manual checklist:** entries from Stage 4's manual runs are visible and
filterable.

---

Out of scope (stays in backlog): `GET /admin/analytics` and dashboard stats
(blocked on tickets), group rename UI, `GroupSelectionPage.jsx` rename.
