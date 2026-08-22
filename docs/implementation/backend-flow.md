# Backend — as built

How the Actix-web backend is actually organized, and what happens to a request
from the socket to the database and back. For the *intended* design, see
[`docs/specification/backend.md`](../specification/backend.md) and
[`rbac.md`](../specification/rbac.md); where the two disagree, the deviations are
listed in [`deviations.md`](deviations.md).

Stack: Rust (edition 2024), actix-web 4, MongoDB driver 3, `jsonwebtoken`,
`bcrypt`, `reqwest` (for Gemini). No ORM — repositories talk to the driver
directly.

---

## Startup

`main.rs` runs a fixed sequence before the server binds:

1. **`Config::from_env`** — loads `.env` via `dotenvy`. `MONGO_URI` and
   `JWT_SECRET` are required and a missing one aborts startup;
   `COOKIE_SECURE` (default `true`), `FRONTEND_ORIGIN` (default
   `http://localhost:5173`) and `GEMINI_API_KEY` are optional. The Gemini key
   is deliberately optional: AI is advisory, so a missing key takes down only
   the AI endpoints (per-request `Internal`), not the whole process.
2. **`db::connect`** — opens the client and issues a `ping`, because the driver
   connects lazily and would otherwise report success against an unreachable
   server. The database is always named `resolve`.
3. **`db::ensure_indexes`** — creates every index in `db.rs`'s `index_table`
   (see [`data-model.md`](data-model.md)). Idempotent, runs on every boot.
4. **One-time migrations** — `backfill_ticket_content_updated_at` and
   `wipe_legacy_chat_messages`. Both are filtered so they are no-ops after the
   first run.
5. **Server** — `AppState { db, config }` is shared as `web::Data`, wrapped in
   `Logger` and a CORS layer. CORS names `frontend_origin` explicitly rather
   than using a wildcard, because the refresh cookie requires credentialed
   requests and the CORS spec forbids combining those with `*`.

Everything is mounted under a single `/api/v1` scope. The bind address is fixed
at `127.0.0.1:8080`.

## Module layout

`lib.rs` / `main.rs` declare one module per feature. Almost every feature module
has the same five files:

| File | Responsibility |
| --- | --- |
| `models.rs` | Documents and request/response DTOs. No behavior. |
| `repository.rs` | MongoDB access only. No authorization, no business rules. |
| `service.rs` | Business logic and every authorization check. |
| `handlers.rs` | Thin Actix handlers: parse input, call the service, map to `HttpResponse`. Format validation (lengths, empty strings) lives here. |
| `mod.rs` | Re-exports. |

The feature modules:

- **`auth`** — registration, login, `/me`, password change, refresh, logout.
  Extra files: `jwt.rs`, `refresh_token.rs`, `password.rs`, `claims.rs`.
- **`user`** — user documents and lookups. No handlers of its own; it exists so
  other modules (auth, rbac, admin, and every service that resolves a display
  name) share one user access path.
- **`group`** — groups and membership: create, rename, delete, list, member
  add/remove/role-change, leave. Also owns `purge_group_data`, the cascade used
  when a group is deleted.
- **`rbac`** — no handlers, no models: just `RbacService`, the shared
  authorization primitives (below).
- **`ticket`** — ticket CRUD, per-group ticket numbering, filtering/search.
- **`comment`** — comments on tickets, including threaded replies.
- **`reaction`** — emoji reactions on comments (`PUT`/`DELETE`, one per user per
  comment).
- **`link`** — typed links between two tickets in the same group.
- **`reference`** — external references attached to a ticket.
- **`activity`** — the per-ticket activity log. Read-only over HTTP; entries are
  written by the services that cause them.
- **`admin`** — System Admin surface: user list, group list, promote/demote,
  user deletion with succession, audit log.
- **`ai`** — Gemini-backed summarize/analyze plus per-user chat conversations.
  Extra file: `client.rs`.

Supporting modules: `config.rs`, `db.rs`, `state.rs`, `errors/`, `server/`
(routes + extractors), and `utils/` (shared `RepoResult`, `insert_id`,
`is_duplicate_key`, regex escaping for user-supplied search terms, and a
Levenshtein distance used for typo-tolerant title search).

## Request lifecycle

```
HTTP request
  → CORS + Logger
  → route match (server/routes.rs)
  → extractor: AuthenticatedUser | GroupScoped | SystemAdminUser
  → handler        (parse + format validation)
  → service        (business rules + RBAC)
  → repository     (Mongo query, always group-scoped)
  → response, or ApiError → JSON error body
```

### Extractors (`server/middleware.rs`)

Authentication is not a middleware wrapper but three `FromRequest` extractors, so
each route declares its own requirement in its handler signature:

- **`AuthenticatedUser`** — decodes the bearer token and yields `user_id`.
  Entirely stateless: signature and `exp` only, no database lookup. That is safe
  because access tokens live 15 minutes; revocation is the refresh token's job.
  Used where authentication alone is the requirement.
- **`GroupScoped`** — the tenant-scoped context. Decodes the token, parses the
  `{id}` path segment as the group id, and calls `RbacService::require_member`.
  Membership is therefore re-checked on every request, so a user removed from a
  group is rejected on their next call rather than at token expiry. Handlers
  scoped to a group take this and never parse a group id themselves.
- **`SystemAdminUser`** — decodes the token and requires the global System Admin
  role. Used by every `/admin` route.

The extractor is the request-level half of RBAC only. Service-layer checks still
run underneath it — both layers are required.

### RBAC (`rbac/service.rs`)

`RbacService` answers one question: what is this user's relationship to this
group, or to the system?

| Method | Rule |
| --- | --- |
| `require_member` | Returns the `GroupMember` (with role) or `Forbidden`. |
| `require_group_admin` | Member *and* role is Group Admin. |
| `require_system_admin` | Global role is System Admin. |
| `require_owner_or_group_admin` | Pure function: Group Admin, or the resource's creator. |

Two deliberate absences: **group isolation** is not enforced here — it is
enforced by every repository query filtering on `group_id` — and the
**sole-Group-Admin succession guard** stays in `GroupService`, since it is
membership business logic rather than a reusable primitive.

Non-membership returns `Forbidden`, never `NotFound`, so a non-member cannot
probe whether a group id exists. The same reasoning applies to a missing user in
`require_system_admin`.

### Errors (`errors/api_error.rs`)

One enum, `ApiError`, implements Actix's `ResponseError` and is the return type
of every handler and service. Every error serializes to the same body:

```json
{ "error": { "code": "forbidden", "message": "..." } }
```

| Variant | Status | Notes |
| --- | --- | --- |
| `InvalidCredentials` | 401 | |
| `Unauthenticated` | 401 | Missing/invalid/expired token. |
| `Forbidden` | 403 | Message is deliberately generic. |
| `NotFound` | 404 | |
| `DuplicateEmail` | 409 | |
| `Conflict(String)` | 409 | Caller-supplied message. |
| `Validation(String)` | 400 | Caller-supplied message. |
| `RateLimited(String)` | 429 | Currently only AI chat. |
| `Internal` | 500 | |

Repository and driver errors never reach the client directly: `From` impls
collapse `mongodb::error::Error`, bcrypt, and JWT failures into `Internal`, while
domain-specific repo enums (`UserRepoError`, `GroupRepoError`, `LinkRepoError`)
map their duplicate-key cases to `DuplicateEmail` / `Conflict`.

## Authentication

Two tokens, with different jobs.

**Access token** — a JWT (`sub` = user id, `exp`), 15-minute TTL, sent by the
client as `Authorization: Bearer`. Verified statelessly.

**Refresh token** — 32 bytes of CSPRNG output, hex-encoded. Only its SHA-256
hash is stored, so a leaked database cannot be used to mint sessions; SHA-256 is
adequate (rather than bcrypt) precisely because the input is already
high-entropy. TTL is 30 days, enforced both by the stored `expires_at` and by a
MongoDB TTL index that reaps spent tokens without a cron job.

It travels in a cookie scoped to `path=/api/v1/auth`, so it is never sent on
unrelated API calls. `httpOnly` keeps it away from JS; `SameSite=Strict` is
fixed (SameSite ignores port and, for same-domain hosts, subdomain — so it
already covers both the local dev split and a production frontend/API subdomain
split); `Secure` is config-driven because browsers refuse Secure cookies over
plain HTTP.

Notable route behaviors:

- `POST /auth/refresh` takes **no** access token — by the time a client needs to
  refresh, its access token has usually expired. The cookie *is* the session
  identifier. Refresh tokens are single-use and rotate on every call.
- `POST /auth/logout` also needs no access token, and revokes only the token in
  this request's cookie, so other devices stay signed in.
- `POST /auth/me/password` revokes every *other* session: the request's own
  refresh cookie is hashed and spared, so changing your password does not log
  out the device you changed it on.

## Route map

| Scope | Guard | Contents |
| --- | --- | --- |
| `/auth` | mixed | `register`, `login` (none); `me`, `me` PATCH, `me/password` (`AuthenticatedUser`); `refresh`, `logout` (cookie only). |
| `/groups` | `AuthenticatedUser` for create/list; `GroupScoped` for everything under `/{id}` | Group CRUD, membership, and all nested resources: `tickets`, `comments`, `reactions`, `activity`, `links`, `references`. |
| `/ai` | `GroupScoped` (`/ai/groups/{id}/tickets/{ticket_id}/…`) | `summarize`, `analyze`, and conversation create/list/delete plus message send/list. |
| `/admin` | `SystemAdminUser` | `users`, `groups`, `groups/{id}` DELETE, `audit-log`, and `users/{id}/{deletion-check,delete,promote,demote}`. |

Every group-scoped path is `/groups/{id}/...`. There is no "active group" on the
server: the group is always named explicitly in the path.

## Cross-cutting behaviors

**Group isolation.** Every repository method takes `group_id` and puts it in the
query filter. A ticket id from another group therefore reads as `NotFound`
rather than leaking that it exists elsewhere. Services that receive both a group
and a nested resource id (comments, activity, links, AI) re-verify that the
ticket belongs to the group, because `GroupScoped` only proves membership in the
group — not that the ticket is in it.

**Ticket numbering.** Each group has its own `ticket_number` sequence, allocated
by an atomic counter (`TicketRepository::next_ticket_number`) and backed by a
unique index on `(group_id, ticket_number)`.

**Activity logging.** Ticket updates diff before/after values and record an event
only when something actually changed. Entries carry the actor id; the display
name is resolved at read time so later renames are reflected.

**Cascade deletes.** Deleting a ticket clears its comments, reactions, activity,
links (both directions, so no dangling relation is left on the other end),
references, and AI insights/conversations. Deleting a group runs
`group::purge_group_data` over the same surface for every ticket in it.

**Uniqueness.** Application-level pre-checks (duplicate email, duplicate member,
duplicate link) exist to produce good error messages; the unique index is what
actually rejects a concurrent duplicate. `utils::is_duplicate_key` recognizes
error code 11000 in both the write-error and command-error shapes.

## AI module

`AiProvider` is a trait, with `GeminiClient` as the production implementation, so
the service can be exercised against a fake without network access. Model is
`gemini-flash-latest` (an alias, not a pinned version — pinned `gemini-2.0-flash`
returned quota-exhausted on this project, and the alias avoids re-pinning when
Google retires dated names). `analyze` requests a JSON response schema so the
output is parsed rather than scraped from prose.

Rules the module holds to:

- **Never writes domain state.** It only reads tickets and writes its own
  `ai_ticket_insights`, `ai_conversations`, and `ai_chat_messages`.
- **Cached where possible.** Summaries and analyses are stored per ticket and
  reused while fresh — freshness is `content_updated_at` on the ticket, so
  editing the title or description invalidates them but a status change does
  not. Responses carry a `cached` flag.
- **Chat is never cached** (each message is genuinely new) and is rate-limited to
  10 user messages per hour per user, returning `RateLimited`. Only the last 20
  messages are replayed as prompt context, bounding cost regardless of thread
  length; the full history is still returned by the API.
- **Conversations are owner-only.** Any member may start one, but no other
  member — not even a Group Admin — can list, read, or delete someone else's.
  This is intentionally stricter than comments, which are group-visible. A
  conversation id belonging to another ticket or group reads as `NotFound`; a
  real ownership mismatch is `Forbidden`.
- Conversation titles are derived from the first message (truncated on a char
  boundary), not generated by a model call.

## Tests

There are currently **no automated tests in the repository** — no `tests/`
directory and no `#[cfg(test)]` modules. Several code comments still refer to
test-only affordances that outlived their tests (`jwt::issue_token_with_exp`,
`AiService::with_provider`, the free functions in `ai/service.rs` kept
unit-testable). Verification today is manual, against a running backend.
