# Implementation Docs

**What the code actually does**, read out of `backend/src/` and `frontend/src/`.

This is the counterpart to [`../specification/`](../specification/), which describes what the
system is *supposed* to do. Where the two disagree, [`deviations.md`](./deviations.md) is the
reconciliation.

---

## Start here

| Document | What it gives you |
|---|---|
| **[`backend-flow.md`](./backend-flow.md)** | What exists, the four core flows, the module dependency map, feature walkthroughs, and a reading order |
| **[`data-model.md`](./data-model.md)** | Collections, relationships and cardinality, referential integrity, isolation model, atomicity |
| **[`frontend-flow.md`](./frontend-flow.md)** | What exists, session/token handling, data-fetching patterns, feature walkthroughs, reading order |
| **[`deviations.md`](./deviations.md)** | 12 mismatches between code and spec, ranked by severity |

## Detail files

**Backend** — [`backend/`](./backend/)
1. [`01-infrastructure.md`](./backend/01-infrastructure.md) — `main.rs`, `config.rs`, `db.rs`, `state.rs`, `routes.rs`, `errors/`, `utils/`
2. [`02-auth.md`](./backend/02-auth.md) — `auth/` and `user/`
3. [`03-rbac-and-middleware.md`](./backend/03-rbac-and-middleware.md) — the three extractors and `RbacService`, plus a route→guard table
4. [`04-groups.md`](./backend/04-groups.md) — `group/`, including the sole-Group-Admin invariant
5. [`05-tickets.md`](./backend/05-tickets.md) — `ticket/`, the atomic counter, and the hybrid search
6. [`06-admin.md`](./backend/06-admin.md) — `admin/` and the user-deletion succession flow

**Database** — [`db/`](./db/)
- [`collections.md`](./db/collections.md) — field-by-field reference
- [`indexes.md`](./db/indexes.md) — all 11 indexes, what each serves, and what's unindexed

**Frontend** — [`frontend/`](./frontend/)
1. [`01-session-and-routing.md`](./frontend/01-session-and-routing.md) — axios interceptors, `AuthContext`, routes, guards, auth forms
2. [`02-groups.md`](./frontend/02-groups.md) — team pages, `useGroup`, `MemberManager`
3. [`03-admin.md`](./frontend/03-admin.md) — admin panels and `DeleteUserModal`
4. [`04-account.md`](./frontend/04-account.md) — profile and password forms
5. [`05-layout-and-ui.md`](./frontend/05-layout-and-ui.md) — layouts, sidebar, UI primitives, utils, styling
6. [`06-libraries.md`](./frontend/06-libraries.md) — every dependency, frontend and backend, and notable absences

---

## The 60-second version

**Stack.** Rust + Actix-web + MongoDB on the backend; React 19 + Vite + Tailwind v4 +
React Query on the frontend. No TypeScript. Layered architecture throughout:
`Handler → Service → Repository → Mongo`.

**Multi-tenancy.** Groups are the tenant boundary. There is no "active group" anywhere —
scope is always the `{id}` path segment. Isolation is enforced twice, by two different
mechanisms: the `GroupScoped` extractor rejects non-members (403), and every tenant-data
query filters on `group_id` so a foreign resource id simply isn't found (404).

**RBAC.** Two independent layers that don't override each other: a global role
(`users.global_role`, System Admin only) and a per-group role (`group_members.role`,
Contributor or Group Admin). Roles are resolved per request and never carried in the JWT, so
a demoted user loses access immediately.

**Sessions.** Two tokens. A 15-minute stateless JWT held in JS memory, and a 30-day
single-use refresh token in an httpOnly `SameSite=Strict` cookie, stored server-side only as
a SHA-256 hash. All revocation lives at the refresh layer.

**Build status.** Auth, groups, RBAC, tickets, and admin are complete on the backend.
**Comments and AI are empty files** — scaffolded, never written. The frontend has no ticket
UI at all, and its "Issues" nav link 404s.
