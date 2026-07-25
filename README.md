![Resolve](/frontend/public/preview.png)

# RESOLVE

A multi-tenant bug tracker built around strict group isolation and two independent
RBAC layers. Rust + Actix-web + MongoDB on the backend, React 19 + Vite + Tailwind on
the frontend.

> **Status: in development.** The core system (auth, groups, RBAC, tickets, admin) is
> complete on the backend. Comments, AI, and the ticket UI are not built yet — see
> [What's not built yet](#whats-not-built-yet).

---

## Core ideas

**Groups are the tenant boundary.** Every ticket and comment belongs to exactly one
group, and no request ever crosses that line. There is no "active group" concept —
scope is always the `{id}` in the request path (`/groups/{id}/...`). Isolation is
enforced twice, by two different mechanisms: a `GroupScoped` extractor rejects
non-members (403), and every tenant-data query also filters on `group_id`, so a
foreign resource id simply isn't found (404).

**Two RBAC layers that don't override each other.** A global role
(`users.global_role`, System Admin only) and a per-group role (Contributor or Group
Admin). System Admin is for system metadata — user list, group list, audit log — and
grants no access to ticket or comment data unless that admin is a member of the group.
Roles are resolved per request and never carried in the JWT, so a demotion takes effect
immediately.

**Sessions use two tokens.** A 15-minute stateless JWT held in JS memory, and a 30-day
single-use refresh token in an httpOnly `SameSite=Strict` cookie, stored server-side
only as a SHA-256 hash. All revocation lives at the refresh layer.

**The backend is the source of truth.** The frontend is UI only; every permission check
is enforced server-side.

---

## Tech stack

| Layer | Choices |
|---|---|
| Backend | Rust (edition 2024), Actix-web 4, MongoDB 3 driver |
| Auth | `jsonwebtoken`, `bcrypt`, SHA-256 hashed refresh tokens |
| Frontend | React 19, Vite 8, React Router 7, TanStack Query 5, Tailwind v4, Radix primitives, lucide-react |
| Database | MongoDB (11 indexes, no transactions) |
| AI (planned) | Gemini API |

No TypeScript anywhere, by decision. Architecture is layered throughout:
`Handler → Service → Repository → Mongo`.

---

## Getting started

**Prerequisites:** Rust (stable), Node 20+, and a MongoDB instance (local or Atlas).

### Backend

```bash
cd backend
cp .env.example .env      # then fill in MONGO_URI and JWT_SECRET
cargo run                 # serves on http://127.0.0.1:8080
```

`.env`:

| Variable | Required | Notes |
|---|---|---|
| `MONGO_URI` | yes | Connection string |
| `JWT_SECRET` | yes | Signing key for access tokens |
| `COOKIE_SECURE` | no | Defaults to `true`; set `false` for local HTTP, or browsers silently drop the refresh cookie |
| `FRONTEND_ORIGIN` | no | Defaults to `http://localhost:5173`. Must be explicit (not `*`) because the refresh cookie requires credentialed CORS |

All routes are served under `/api/v1`.

### Frontend

```bash
cd frontend
npm install
cp .env.example .env      # VITE_API_URL=http://localhost:8080/api/v1
npm run dev               # serves on http://localhost:5173
```

### Tests

134 backend tests (unit + API-level, hitting a real MongoDB).

```bash
cd backend
cargo test -- --test-threads=1
```

`--test-threads=1` is required — the tests share database collections and will
interfere with each other in parallel. There are no automated frontend tests; the
frontend is verified manually.

---

## API surface

Everything under `/api/v1`.

**Auth** — `POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`,
`POST /auth/logout`, `GET /auth/me`, `PATCH /auth/me`, `POST /auth/me/password`

**Groups** — `POST /groups`, `GET /groups`, `GET|PATCH|DELETE /groups/{id}`,
`GET|POST /groups/{id}/users`, `GET /groups/{id}/users/lookup`,
`PATCH|DELETE /groups/{id}/users/{user_id}`

**Tickets** — `POST|GET /groups/{id}/tickets`,
`GET|PATCH|DELETE /groups/{id}/tickets/{ticket_id}` (with filters, pagination, and a
hybrid text/fuzzy search)

**Admin** — `GET /admin/users`, `GET /admin/groups`, `DELETE /admin/groups/{id}`,
`GET /admin/audit-log`, `GET /admin/users/{id}/deletion-check`,
`POST /admin/users/{id}/delete`

Full request/response detail lives in [`docs/specification/api.md`](docs/specification/api.md).

---

## What's built

- **Auth** — registration, login, refresh rotation, logout, profile update, password change (revokes other sessions)
- **Groups** — create, rename, delete, membership management, role changes, member lookup by email, and the sole-Group-Admin invariant (a group can never be left without an admin)
- **RBAC** — three extractors plus a service layer, enforced on every request
- **Tickets (backend)** — full CRUD, per-group sequential numbering via an atomic counter, filtering, pagination, hybrid search
- **Admin** — user and group listings with search, audit log, and the user-deletion flow with explicit successor appointment
- **Frontend** — landing page with a live product demo, auth flows, dashboard, team management workspace, admin panel, account settings, sidebar app shell, shared UI primitives

## What's not built yet

Tracked in detail in [`docs/implementation/deviations.md`](docs/implementation/deviations.md).

| Area | State |
|---|---|
| **Comments** | Not started. `backend/src/comment/*` and `frontend/src/features/comments/*` are empty files; no routes, no collection. `RbacService::require_owner_or_group_admin` is written and waiting for it. |
| **AI (Gemini)** | Not started. `backend/src/ai/*` is empty, `/ai` is registered with no routes inside it, and there is no HTTP client dependency yet. Deliberate sequencing — AI comes after the core system is stable. |
| **Ticket UI** | Not started. The backend is complete, but `TicketsPage`, `TicketDetailPage`, and `features/tickets/*` are empty. The "Issues" nav link currently 404s. |
| **Dashboard stats** | `DashboardStats.jsx` is empty; the dashboard is a placeholder. |
| **`GET /admin/analytics`** | Specified, not implemented. |

Known bugs, also in `deviations.md`: deleting a group orphans its tickets and counter
document; a System Admin can delete their own account despite the UI hiding the button;
plus assorted minor rough edges (byte-vs-char title length, non-idempotent role updates,
uncapped group names).

Naming note: the UI says "Teams" while the backend, API, and database say "group". That
is a deliberate UI-only rename, not drift.

---

## Documentation

Two sets, deliberately kept apart:

- [`docs/specification/`](docs/specification/) — what the system is *supposed* to do
  (architecture, backend, frontend, api, database, rbac, ai-integration)
- [`docs/implementation/`](docs/implementation/) — what the code *actually* does, read
  out of the source. Start with
  [`backend-flow.md`](docs/implementation/backend-flow.md),
  [`frontend-flow.md`](docs/implementation/frontend-flow.md), and
  [`data-model.md`](docs/implementation/data-model.md)
- [`docs/implementation/deviations.md`](docs/implementation/deviations.md) — the
  reconciliation between the two

`CLAUDE.md` holds the working rules for AI-assisted development on this repo.

---

## Repository layout

```
backend/          Rust + Actix-web API
  src/            auth, user, group, ticket, rbac, admin, comment*, ai*, server, errors, utils
  tests/          134 tests, require a live MongoDB
frontend/         React + Vite client
  src/            pages, features, components, hooks, services, lib
docs/             specification/ and implementation/
design/           feature design notes
plan/             branch and distribution planning
```

`*` = scaffolded, not implemented.

---

Ort final project.
