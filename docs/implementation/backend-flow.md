# Backend Flow — Study Guide

Everything here is derived from reading the code in `backend/src/`, not from
`docs/specification/`.
Where the two disagree, see [`deviations.md`](./deviations.md).

Detailed per-file breakdowns live in [`backend/`](./backend/):

| File | Covers |
|---|---|
| [`backend/01-infrastructure.md`](./backend/01-infrastructure.md) | `main.rs`, `lib.rs`, `config.rs`, `db.rs`, `state.rs`, `server/routes.rs`, `errors/`, `utils/` |
| [`backend/02-auth.md`](./backend/02-auth.md) | `auth/`, `user/` |
| [`backend/03-rbac-and-middleware.md`](./backend/03-rbac-and-middleware.md) | `server/middleware.rs`, `rbac/` |
| [`backend/04-groups.md`](./backend/04-groups.md) | `group/` |
| [`backend/05-tickets.md`](./backend/05-tickets.md) | `ticket/` |
| [`backend/06-admin.md`](./backend/06-admin.md) | `admin/` |

---

## 0. What actually exists

Read this first — it saves you from explaining features that aren't there.

**Built and working:** auth (register/login/refresh/logout/profile/password), users,
groups + membership + roles, RBAC enforcement, tickets (full CRUD + search + filters +
pagination), system-admin panel (user list, group list, user deletion with succession,
audit log).

**Scaffolded but empty (0 bytes):** `src/comment/*.rs` and `src/ai/*.rs` — every file in
those two modules is an empty file. `main.rs` declares `mod comment;` and `mod ai;`, and
`routes.rs` registers `web::scope("/ai")` with **no routes inside it**. So comments and
AI are declared in the module tree and reachable in the docs, but there is zero
implementation. The Gemini integration that `CLAUDE.md` calls "the core system feature"
does not exist in code.

This matches the build order in `CLAUDE.md`: steps 1–5 done except comments, step 6
(frontend) partial, step 7 (AI) not started.

---

## 1. The basic flows (short version)

These four are the spine of the system. If you can explain these, you can explain the backend.

### Flow A — Process startup

`main.rs::main`
1. `Config::from_env()` — loads `.env`, reads `MONGO_URI`, `JWT_SECRET`, `COOKIE_SECURE`, `FRONTEND_ORIGIN`. Missing required var → process exits.
2. `db::connect()` — builds a Mongo `Client` and immediately issues `{ping: 1}`, because the driver connects lazily and would otherwise report success without a real connection.
3. `db::database()` — hardcoded database name `"resolve"`.
4. `db::ensure_indexes()` — creates every index at boot (idempotent).
5. Builds `AppState { db, config }`, wraps it in `web::Data` (an `Arc`), and shares one instance across all worker threads.
6. `HttpServer::new(...)` runs the closure **once per worker thread**, each building an `App` with: CORS layer (explicit origin, credentials enabled), `Logger`, and everything mounted under `web::scope("/api/v1")`.
7. Binds `127.0.0.1:8080` (hardcoded in `Config::bind_address`).

### Flow B — Any authenticated request

```
HTTP request
  → Actix router matches a route in server/routes.rs
  → Extractor runs (AuthenticatedUser | GroupScoped | SystemAdminUser)
        - parses "Authorization: Bearer <jwt>"
        - verifies JWT signature + exp (no DB hit)
        - for GroupScoped: reads {id} from the path, DB lookup of group_members → role
        - for SystemAdminUser: DB lookup of users.global_role
  → Handler (handlers.rs) — validates the request body/query shape only
  → Service (service.rs) — re-runs the RBAC check, applies business rules
  → Repository (repository.rs) — the only place that touches MongoDB
  → Response, or ApiError → JSON { error: { code, message } }
```

The layer split is strict: handlers never touch Mongo, repositories never make
authorization decisions. Services are the only layer that does both.

### Flow C — Session lifecycle (the two-token model)

Two tokens with different jobs:

- **Access token (JWT)** — 15 minutes, stateless, sent as `Authorization: Bearer`. Never revoked individually; a stolen one just expires.
- **Refresh token** — 30 days, random 32 bytes, delivered as an httpOnly `SameSite=Strict` cookie scoped to `/api/v1/auth`. Stored server-side only as a SHA-256 hash. **Single-use**: `POST /auth/refresh` revokes the presented token and issues a new one.

Revocation therefore lives entirely at the refresh-token layer. Logout revokes one
token (per-device). Password change revokes all *other* tokens for that user.

### Flow D — Multi-tenancy (group isolation)

There is no "active group" anywhere — not in the JWT, not in server state. Scope is
always the `{id}` path segment. Isolation is enforced in **two independent places**:

1. **RBAC** — `GroupScoped` extractor + `RbacService::require_member` reject a non-member with 403 (deliberately 403, not 404, so a non-member cannot probe whether a group id exists).
2. **Repository filters** — every tenant-data query includes `group_id` in the filter document. `TicketRepository::find_by_id(group_id, ticket_id)` filters on **both**, so a valid ticket id paired with the wrong group id simply matches nothing → 404.

These are separate mechanisms: #1 stops the wrong *person*, #2 stops the wrong *id*.

---

## 2. Module dependency map

Arrows mean "calls into". Nothing here is circular.

```
                      config.rs ── db.rs ── state.rs (AppState { db, config })
                                                │
                              server/routes.rs ─┴─ server/middleware.rs
                                    │                     │
        ┌───────────┬───────────────┼──────────┐          │
        │           │               │          │          │
   auth/handlers group/handlers ticket/handlers admin/handlers
        │           │               │          │          │
   auth/service  group/service  ticket/service admin/service
        │           │  │  │          │  │  │      │  │  │
        │           │  │  └──────────┼──┼──┼──────┘  │  │
        │           │  │             │  │  │         │  │
        │           │  └── ticket/repository         │  │
        │           │                   │            │  │
        │           └───────────────────┴────────────┘  │
        │                    group/repository            │
        │                          ▲                     │
        │                          │                     │
        └── user/service ──────────┴──── rbac/service ────┘
                 │                              │
           user/repository              admin/repository
                                        (audit log only)
```

Things worth knowing about this graph:

- **`rbac/service.rs` is the shared hub.** It owns `GroupRepository` + `UserService` and answers only one kind of question: "what is this user's relationship to this group / to the system?" Every feature service holds an `RbacService`.
- **`group/service.rs` depends on `ticket/repository.rs`** — the only cross-feature dependency in the whole backend. It exists solely so `GET /groups` can report `open_ticket_count` per group.
- **`admin/service.rs` bypasses `GroupService`** and talks to `GroupRepository` directly. Deliberate: the admin succession flow must do things `GroupService` forbids (reassigning roles it doesn't own), so it can't route through the group business rules.
- **`errors/api_error.rs` depends on every repository's error enum** via `From` impls. That's what makes `?` work uniformly across all layers.
- **Services are constructed per-request**, e.g. `GroupService::new(&state.db)` inside the handler. They're cheap (just `Collection` handles, which are themselves cheap clones over the shared connection pool). There's no DI container.

### `lib.rs` vs `main.rs`

The crate builds twice. `main.rs` is the binary and declares `mod ai; mod comment; mod db;`
plus the rest. `lib.rs` is the library used by the integration tests in `backend/tests/`,
and exports a **subset**: `admin, auth, config, errors, group, rbac, server, state, ticket,
user, utils` — no `ai`, no `comment`, no `db`. Tests build their own Mongo client via
`tests/support/mod.rs` instead of using `db::connect`.

---

## 3. Feature flows, end to end

### Register

`POST /api/v1/auth/register` → `auth::handlers::register`
1. `validate_register` — email contains `@`, password ≥ 8 chars, name non-blank.
2. `AuthService::register` → `password::hash_password` (bcrypt cost 12) → `UserService::create` → `UserRepository::create` (insert into `users`).
3. Unique index on `users.email` is what actually prevents duplicates; a duplicate-key error (code 11000) is mapped to `UserRepoError::DuplicateEmail` → `409 duplicate_email`.
4. `AuthService::issue_session` — mints the JWT and inserts a `refresh_tokens` row.
5. Handler returns `201` with `{ user, jwt }` and sets the refresh cookie.

### Login

Same tail as register, different head: `UserService::find_by_email` → `password::verify_password`.
Both a missing user and a wrong password return the identical `InvalidCredentials` error, so the endpoint doesn't leak which emails are registered.

### Refresh (token rotation)

`POST /auth/refresh` takes **no** `AuthenticatedUser` — by design, since the access token
is usually already expired when a client needs to refresh.
1. Read raw token from cookie.
2. `refresh_token::hash_token` → SHA-256.
3. `AuthRepository::find_active_by_hash` — filter is `{token_hash, revoked_at: null, expires_at: {$gt: now}}`. A replayed (already-rotated) token fails this filter naturally; there's no separate reuse-detection code.
4. Revoke the found row by `_id`.
5. `issue_session` for the same user → new JWT + new refresh row + new cookie.

### Create group

`POST /groups` → `GroupService::create_group`: inserts into `groups`, then inserts a
`group_members` row for the creator with `role: GroupAdmin`. **Two sequential writes, not a
transaction** (documented as an accepted risk in the code comment). `owner_id` is stored on
the group but is never used for authorization — every check reads `group_members.role`.

### Create ticket

`POST /groups/{id}/tickets` → `GroupScoped` extractor (must be a member) → `validate_create`
(title non-blank ≤ 200 chars, description non-blank) → `TicketService::create_ticket`:
1. `RbacService::require_member` again (defense in depth).
2. `TicketRepository::next_ticket_number` — `find_one_and_update` with `$inc` + `upsert` on the `counters` collection, keyed by `_id == group_id`. This is atomic, so two simultaneous creations in one group cannot collide.
3. Insert the ticket with `status: Open` and `created_by` from the token — neither is accepted from the client.
4. `enrich_ticket` — one extra `users` lookup to attach `created_by_name`.

### List tickets (the interesting one)

`GET /groups/{id}/tickets?q=&status=&priority=&creator=&page=&per_page=`

Split between Mongo and the process:
- `status`, `priority`, `creator` are indexable exact-match fields → filtered **in Mongo** by `TicketRepository::list_by_group`.
- `q` (free-text title search) has no Mongo-native equivalent here → done **in-process** by `TicketService::search_by_title`: case-insensitive substring first; if that yields nothing, fall back to Levenshtein distance against each word of each title, allowing `max(len/3, 1)` edits, sorted by distance.
- **Pagination is applied after both**, in memory (`.skip(start).take(per_page)`), so `total` is the post-search count. `per_page` defaults to 20, clamped to 1..=100.

Note the cost model: the whole filtered set is pulled into memory before paging. Fine at
current scale, but it's the one place the backend doesn't push work down to the database.

### Delete a user as System Admin (the most complex flow)

Two endpoints, one plan-and-commit shape:

`GET /admin/users/{id}/deletion-check` → `AdminService::deletion_check` → `build_plan`.
`build_plan` walks every group the target belongs to and sorts each into one of three buckets:
- **`plain_removals`** — target is a Contributor, or a Group Admin alongside other admins. Just drop the membership.
- **`auto_delete`** — target is sole Group Admin *and* the only member. The group gets deleted.
- **`blocked`** — target is sole Group Admin but other members exist. Requires an explicitly named successor.

`POST /admin/users/{id}/delete` → `AdminService::delete_user`:
1. **Re-derives the plan from scratch** rather than trusting the client's copy — membership can shift between check and commit.
2. Validates every blocked group has a successor, and that each named successor is still a member. Any failure → `409`, **before any write happens**.
3. Then executes: promote successor → remove target's membership → write audit entry; for auto-delete groups: delete members → delete group → write audit entry; then plain removals; **user document deleted last**, so a mid-failure retry is always safe.

Not a Mongo transaction — sequential writes, ordered so partial failure is recoverable.
Audit entries snapshot names (`group_name`, `deleted_user_name`, ...) at write time,
because the referenced entities won't exist to look up later.

---

## 4. Suggested reading order

If you're studying this to explain it, read in this order — each file only depends on ones above it.

1. `config.rs`, `state.rs`, `db.rs` — 200 lines total, gives you the whole environment.
2. `errors/api_error.rs` — the error vocabulary every other file speaks.
3. `server/routes.rs` — the complete API surface on one screen.
4. `auth/jwt.rs`, `auth/password.rs`, `auth/refresh_token.rs` — three tiny, self-contained crypto helpers.
5. `server/middleware.rs` — the three extractors. This is where authentication becomes authorization.
6. `rbac/service.rs` — 88 lines, four functions, referenced by everything.
7. `user/` then `auth/` — the simplest full vertical slice (handler → service → repo).
8. `group/` — the first module with real business rules (sole-admin guard).
9. `ticket/` — the first module with real query complexity (search, filters, paging, counters).
10. `admin/` — the hardest, and it assumes you know groups already.
