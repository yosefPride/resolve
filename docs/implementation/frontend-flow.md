# Frontend Flow — Study Guide

Derived from the code in `frontend/src/`, not from `docs/specification/frontend.md`.
Where the two disagree, see [`deviations.md`](./deviations.md).

Detail files in [`frontend/`](./frontend/):

| File | Covers |
|---|---|
| [`frontend/01-session-and-routing.md`](./frontend/01-session-and-routing.md) | `main.jsx`, `App.jsx`, `lib/axios.js`, `lib/AuthContext.jsx`, route guards, auth forms |
| [`frontend/02-groups.md`](./frontend/02-groups.md) | `features/groups/`, `hooks/useGroup.js`, `pages/GroupManagementPage.jsx` |
| [`frontend/03-admin.md`](./frontend/03-admin.md) | `features/admin/`, `features/users/`, `pages/AdminPage.jsx` |
| [`frontend/04-account.md`](./frontend/04-account.md) | `features/account/`, `pages/AccountPage.jsx` |
| [`frontend/05-layout-and-ui.md`](./frontend/05-layout-and-ui.md) | `components/layout/`, `components/ui/`, `components/marketing/`, `utils/` |
| [`frontend/06-libraries.md`](./frontend/06-libraries.md) | Every external dependency, frontend and backend |

---

## 0. What actually exists

The frontend is **behind the backend**. Tickets are fully built server-side and have no UI at all.

**Built:** landing page, register/login, session bootstrap + silent refresh, dashboard
(a stub), account page (profile + password), team management (members, roles, rename,
delete, leave), admin panel (users, teams, audit log, user deletion with succession),
sidebar/app shell.

**Empty files (0 bytes) — scaffolded, never written:**
```
pages/TicketsPage.jsx          features/tickets/*  (5 files)
pages/TicketDetailPage.jsx     features/comments/* (2 files)
hooks/useTickets.js            features/ai/*       (3 files)
hooks/useComments.js           features/dashboard/DashboardStats.jsx
hooks/useAI.js                 components/layout/PageWrapper.jsx
services/tickets.service.js    services/comments.service.js
services/ai.service.js         services/users.service.js
lib/auth.js                    utils/validators.js
```

Two consequences you'll notice immediately when running the app:

1. **The "Issues" nav link is broken.** Both `Header.jsx` and `Sidebar.jsx` link to `/tickets`, but `App.jsx` registers no such route — clicking it hits the `*` catch-all and renders `NotFoundPage`.
2. **`DashboardPage` is a placeholder** — a greeting and a logout button, nothing else.

---

## 1. The basic flows (short version)

### Flow A — App boot and session restoration

The single most important flow in the frontend, because everything else assumes it finished.

```
main.jsx
  → QueryClientProvider (React Query, retry: 1)
    → BrowserRouter
      → AuthProvider          ← blocks the whole app while status === 'loading'
        → App (routes)
```

`AuthProvider` on mount:
1. Registers an unauthorized handler with the axios module.
2. Calls `bootstrapSession()` → `POST /auth/refresh` → store JWT in memory → `GET /auth/me`.
3. Success → `user` set, `status: 'authenticated'`. Failure → `status: 'unauthenticated'`.
4. **While `status === 'loading'` it renders `<Spinner />` and nothing else** — no route renders until the session question is settled, which is what prevents a logged-in user from flashing the login page on refresh.

`bootstrapPromise` is **module-scoped, not component state**. React StrictMode double-invokes
effects in dev; since refresh tokens are single-use, a genuine second `POST /auth/refresh`
would 401 and flip a valid session to logged-out. Hoisting the promise out of the component
makes the second invocation reuse the first call.

### Flow B — Token handling on every request

`lib/axios.js` holds the access token in a **module-level variable** — never `localStorage`,
never `sessionStorage`. The reasoning: the refresh token is deliberately httpOnly to keep it
away from JS, so leaving the access token in a JS-readable store would undo that.

- **Request interceptor** — attaches `Authorization: Bearer <token>` when a token is set.
- **Response interceptor** — on a `401` that isn't already a retry and isn't on `/auth/login`, `/auth/register`, or `/auth/refresh`: refresh once, replay the original request with the new token. If the refresh itself fails: clear the token and invoke the unauthorized handler (which resets auth state and navigates to `/login`).
- **`refreshPromise` deduplication** — concurrent 401s share one in-flight refresh. Without it, the second request would present an already-rotated (revoked) token and fail.
- **`withCredentials: true`** — required for the refresh cookie to be sent/received cross-origin.
- **`timeout: 15000`** — a dropped network would otherwise leave a request pending forever, hanging the UI on "Loading…". A timeout rejects with no `error.response`, which is the same shape `utils/errors.js` already handles.

### Flow C — Data fetching

Two coexisting patterns, and knowing which is which saves confusion:

**React Query** — used for anything shared or cached: `['groups']` (sidebar + group stats),
`['group', id]` / `['group', id, 'members']`, `['admin', 'users', search]`,
`['admin', 'groups', search]`, `['admin', 'auditLog']`, `['admin', 'deletionCheck', userId]`.
Mutations invalidate the relevant key rather than manually refetching.

**Plain `useState` + `async` handlers** — used for one-shot forms with no shared state:
login, register, profile, password change, create/rename team, and the delete/leave
confirmations on the group page.

The dividing line is roughly "does anything else on screen need this data?"

### Flow D — Authorization in the UI

Purely cosmetic, and the code says so repeatedly. Three layers:

1. **`ProtectedRoute`** — `status === 'unauthenticated'` → `<Navigate to="/login" replace />`.
2. **`AdminRoute`** — wraps `ProtectedRoute`, then `isSystemAdmin(user)` → else `<Navigate to="/dashboard" replace />`.
3. **Conditional rendering** — e.g. `MemberManager` only renders the actions menu and add-member form when `iAmAdmin`.

The backend re-enforces every one of these. `AdminRoute`'s own comment states the guard is
UI-only.

---

## 2. Directory map and dependency direction

```
main.jsx ──── App.jsx ──── pages/*  ──── features/*  ──── services/*  ──── lib/axios.js
   │             │             │              │
   │             │             └──────────────┴──── components/ui/*
   │             │                                  components/layout/*
   │             └──── lib/ProtectedRoute, lib/AdminRoute
   │
   └──── lib/AuthContext.jsx ──── services/auth.service.js
                │
                └──── lib/authContext.js  (the bare createContext)
                          ▲
                          └──── hooks/useAuth.js
```

Layer rules, consistently followed:

- **`services/*`** are the only modules that import `lib/axios`. Each exports plain async functions returning `res.data`. No React, no state.
- **`features/*`** are feature-scoped components. They may call services directly or via React Query.
- **`pages/*`** compose features and own page-level layout. They are thin.
- **`components/ui/*`** are presentational primitives with no data access.
- **`hooks/*`** wrap React Query or pure state logic.

### The `authContext.js` / `AuthContext.jsx` split

Two files that look redundant but aren't:
- `lib/authContext.js` — just `export const AuthContext = createContext(null)`.
- `lib/AuthContext.jsx` — the `AuthProvider` component.

The split exists for **React Fast Refresh**: a module exporting both a component and a
non-component breaks HMR boundaries. Keeping the bare context in its own file lets
`hooks/useAuth.js` import it without pulling in the provider.

---

## 3. Feature flows, end to end

### Register / Login
`RegisterForm` / `LoginForm` hold local form state → call `register`/`login` from `useAuth()`
→ `AuthProvider` calls the service, stores the JWT via `setAccessToken`, sets `user` and
`status` → the form navigates to `/dashboard`. Errors go through `errorMessage(err, fallback)`.

Note these two mutate auth state through the **context**, not the service, so the session
lives in exactly one place.

### Logout
`useAuth().logout()` → `POST /auth/logout` (clears the httpOnly cookie server-side) →
`setAccessToken(null)` → clear `user` → `status: 'unauthenticated'` → `navigate('/login')`.

### Create a team
`Sidebar`'s `TeamsSection` renders a `+` button → `Modal` with `CreateGroupForm` →
`POST /groups` → `onCreated` invalidates `['groups']`, closes the modal, and expands the
section so the new team is visible.

### View / manage a team
`/groups/:id` → `GroupManagementPage` → `useGroup(id)` fires two parallel queries
(`GET /groups/:id` and `GET /groups/:id/users`) under a shared `['group', id]` prefix.

The page derives `myRole` by finding **its own user in the members list** — the backend does
return the caller's role via `GET /groups` (`GroupSummaryResponse.role`), but this page uses
the members array it already has. From `iAmAdmin` it branches: admins get rename + delete;
non-admins get "Leave team".

`MemberManager` handles add (email lookup → confirm → add with a role), promote/demote, and
remove/leave. Every mutation invalidates `['group', id, 'members']`.

### Admin: delete a user
`UsersPanel` → `DeleteUserModal`, which mirrors the backend's plan-then-commit shape:
`GET /admin/users/:id/deletion-check` → render a `<select>` per blocked group and a warning
per auto-delete group → `POST /admin/users/:id/delete` with the chosen successors.
A `409` (server re-derived a different plan) shows the message **and refetches the check in
place** without closing the modal.

The `chosenSuccessor(group)` helper **derives** validity instead of syncing state: it returns
a stored pick only if that person is still in `eligible_successors`. After a 409 re-check,
a now-invalid pick silently falls back to empty and re-disables the submit button — no
`useEffect` pruning required.

---

## 4. Conventions worth knowing

**"Groups" in the API, "Teams" in the UI.** The backend, routes (`/groups/:id`), services,
and query keys all say *group*; every user-facing string says *Team* ("Team Admin", "Create
team", "No teams yet"). This is an intentional UI-only rename, not drift. Likewise
*tickets* → "Issues" in nav labels while the route stays `/tickets`.

**Error handling is uniform.** `utils/errors.js::errorMessage(err, fallback)` returns a
network message when there's no `err.response` (covers offline, CORS, and the 15s timeout),
otherwise `err.response.data.error.message`, otherwise the fallback. A few places branch on
`err.response.data.error.code` first to attach a *field-level* error instead — `ProfileForm`
on `duplicate_email`, `ChangePasswordForm` on `invalid_credentials`.

**Roles are string comparisons in `utils/roles.js`**, and the two conventions differ:
group roles are snake_case (`'group_admin'`), the global role is PascalCase
(`'SystemAdmin'`). That mirrors the backend's serde attributes exactly — `Role` has
`rename_all = "snake_case"`, `GlobalRole` has no rename.

**No automated tests.** No Vitest, no Testing Library, no test files. Verification is manual.

**No TypeScript**, per `CLAUDE.md`. Plain JSX with no prop-types either.

---

## 5. Suggested reading order

1. `lib/axios.js` — 85 lines, and the interceptor logic explains most of the session model.
2. `lib/AuthContext.jsx` + `lib/authContext.js` + `hooks/useAuth.js` — the session state machine.
3. `App.jsx` — the whole route table, plus `ProtectedRoute` / `AdminRoute`.
4. `utils/errors.js`, `utils/roles.js`, `utils/format.js` — 50 lines total, used everywhere.
5. `services/*.js` — the complete API surface the frontend actually uses.
6. `features/auth/` — the simplest full feature (form → context → service).
7. `hooks/useGroup.js` + `pages/GroupManagementPage.jsx` + `features/groups/` — the first real React Query usage.
8. `features/admin/DeleteUserModal.jsx` — the most complex component in the app.
9. `components/layout/Sidebar.jsx` — the largest file (321 lines), and mostly presentational.
