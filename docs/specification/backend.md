# Backend Architecture

---

# Stack

- Rust
- Actix-web
- MongoDB
- JWT
- bcrypt
- Gemini API

---

# Core Design

The backend follows a simple layered architecture:

Request → Handler → Service → Repository → Database

---

# Core Modules

src/

    admin/
    ai/
    auth/
    comment/
    config.rs
    errors/
    group/
    rbac/
    server/
    state.rs
    ticket/
    user/
    utils/
    main.rs

Each module follows a feature-based architecture.

---

## Module Pattern

Each feature should follow:

- handlers.rs
- service.rs
- repository.rs
- models.rs
- mod.rs

---

## Layer Responsibilities

### Handlers

- HTTP endpoints
- Input validation
- Calls service layer

### Services

- Business logic
- Authorization rules
- AI integration orchestration

### Repositories

- MongoDB access only
- No business logic allowed
- MUST include group_id filter

---

# Group Scoping

There is NO "active group" stored anywhere — not in the JWT, not in server
state.

- Every group-scoped route carries the group id in its path (`/groups/{id}/...`)
- The `GroupScoped` request extractor (src/server/middleware.rs) reads that id,
  verifies the caller's membership, and resolves their current role — one
  lookup, per request
- Handlers receive the resolved `{ user_id, group_id, role }` and never parse a
  group id themselves; repositories are always given `group_id` from it
- Because scope is explicit per request and role is never baked into the token,
  there is nothing to "switch" and nothing that can go stale

---

# Session Invalidation

- Access tokens (JWT) are verified statelessly (signature + exp only, no DB
  lookup) and are short-lived (15 minutes), so they are never individually
  revoked — a stolen one simply expires on its own shortly after
- Session-level revocation lives in the refresh token instead: each refresh
  token is a random value, stored server-side only as a SHA-256 hash, in the
  `refresh_tokens` collection
- Refresh tokens are single-use — rotated (revoked, replaced) on every
  `POST /auth/refresh` — and revoked on `POST /auth/logout`
- Revocation is per-session (per refresh token), not per-user — logging out
  on one device does not invalidate other devices' sessions
- The one deliberate per-user case is a password change (`POST /auth/me/password`):
  it revokes every *other* refresh token for the user, sparing only the session
  that made the change (identified by its own cookie). See
  `AuthRepository::revoke_all_for_user_except`
- See docs/database.md ("refresh_tokens") and docs/api.md for details

---

# Request Lifecycle

1. JWT validation (authentication)
2. User extraction
3. Group resolution from the path + membership/role lookup (GroupScoped
   extractor), or global-role check (SystemAdminUser extractor) on /admin routes
4. RBAC check (request-level, via the extractor above)
5. Handler execution
6. Service logic (re-runs the RBAC check — see "RBAC System" below)
7. Repository access (scoped by group)

---

# Group Isolation Rule

EVERY database query MUST include group_id filter.

No exceptions.

---

# RBAC System

- Static role system: Contributor / Group Admin (group-scoped), System Admin (global-scoped)
- No dynamic policy engine
- Enforced in two layers (both always run — defense in depth, per docs/rbac.md):
  - Request level: the `GroupScoped` / `SystemAdminUser` extractors
    (src/server/middleware.rs) reject unauthorized requests before the handler
  - Service level: services call the shared `RbacService`
    (src/rbac/service.rs) helpers — `require_member`, `require_group_admin`,
    `require_system_admin`, `require_owner_or_group_admin`
- Group isolation itself is separate from these role checks: it is the
  repository rule that every group-scoped query filters by `group_id`, so a
  resource id from another group simply isn't found

---

# AI Integration

AI is a core feature but treated as a service dependency:

- Ticket summarization (sync)
- Bug classification (sync)
- Group reports (async recommended)

Rules:

- AI never writes to DB
- AI never bypasses RBAC
- AI always scoped to the group named in the request path

---

# Group Module

Responsible for:

- creating groups
- managing members
- enforcing ownership rules
- enforcing the "at least one Group Admin per group" invariant

Group creation is open to any authenticated user. The creator is automatically assigned the Group Admin role for that group (a group_members row is created in the same operation). A Group Admin may not leave or be removed while they are the group's sole Group Admin — a successor must be appointed first, unless the group is deleted entirely.

---

# System Admin Capabilities

System Admin can:

- view all groups (metadata only)
- view users
- delete users (naming a successor Group Admin where required — see docs/rbac.md, "Group Admin Succession")
- delete groups (no membership or successor check needed — see docs/rbac.md, "Group Admin Succession")
- view system analytics (aggregated only)

System Admin cannot:

- Read tickets across groups
- Access private group data without membership
- Change group roles or membership outside the scoped user-deletion succession exception above
