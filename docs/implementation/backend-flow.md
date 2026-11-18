# Backend — as built

How the Node.js backend is actually organized, and what happens to a request
from the socket to the database and back. For the *intended* design, see
[`docs/specification/backend.md`](../specification/backend.md) and
[`rbac.md`](../specification/rbac.md); where the two disagree, the deviations are
listed in [`deviations.md`](deviations.md).

Stack: Node.js 22+ (plain JavaScript, ES modules), the built-in `node:http`
server — no web framework — the official MongoDB driver, `bcryptjs`, and the
built-in `fetch` (for Gemini). No ORM — repositories talk to the driver
directly.

The backend was translated from an earlier Rust/actix-web implementation, and
keeps that server's HTTP behavior exactly — same routes, status codes, JSON
bodies, cookies, CORS headers, and error texts — and the same stored document
shapes, so it runs against an existing database unchanged. The few modules that
exist only to preserve that behavior are described under
[*Compatibility layer*](#compatibility-layer).

---

## Startup

`main.js` runs a fixed sequence before the server binds:

0. **`initLogger`** (`logging.js`) — with `LOG_LEVEL` unset the threshold is
   error-only, so failures show and routine traffic stays quiet; set
   `LOG_LEVEL=info` for the per-request access log. A failed log write (say,
   the reader of a piped stderr went away) is dropped rather than crashing the
   process.
1. **`Config.fromEnv`** — loads the first `.env` found in the working directory
   or a parent. `MONGO_URI` and `JWT_SECRET` are required and a missing one
   aborts startup; `COOKIE_SECURE` (default `true`), `FRONTEND_ORIGIN` (default
   `http://localhost:5173`), `GEMINI_API_KEY`, `MONGO_DB` (default `resolve`)
   and `PORT` (default `8080`) are optional. The Gemini key is deliberately
   optional: AI is advisory, so a missing key takes down only the AI endpoints
   (per-request `Internal`), not the whole process. That `Internal` is
   byte-identical to the one an upstream provider failure produces, so
   `AiService.create` logs an explicit "GEMINI_API_KEY is not set" to tell the
   two apart.
2. **`db.connect`** (`db/index.js`) — opens the client and issues a `ping`,
   because the driver connects lazily and would otherwise report success against
   an unreachable server.
3. **`db.ensureIndexes`** — creates every index in the `INDEXES` table (see
   [`data-model.md`](data-model.md)). Idempotent, runs on every boot.
4. **One-time migrations** — `backfillTicketContentUpdatedAt` and
   `wipeLegacyChatMessages`. Both are filtered so they are no-ops after the
   first run.
5. **Server** — `createServer({ db, config })` (`server/app.js`) handles CORS,
   routing, extractors, and response writing. CORS names `FRONTEND_ORIGIN`
   explicitly rather than using a wildcard, because the refresh cookie requires
   credentialed requests and the CORS spec forbids combining those with `*`.

Everything is mounted under `/api/v1`. The server binds to `0.0.0.0:$PORT`.

## Module layout

`src/` has one folder per feature. Almost every feature folder has the same four
files:

| File | Responsibility |
| --- | --- |
| `models.js` | Stored-document decoders, response shapes, and request-body types. No behavior. |
| `repository.js` | MongoDB access only. No authorization, no business rules. |
| `service.js` | Business logic and every authorization check. |
| `handlers.js` | Thin handlers: declare extractors, parse ids, call the service, build the `HttpResponse`. Format validation (lengths, empty strings) lives here. |

The feature modules:

- **`auth`** — registration, login, `/me`, password change, refresh, logout.
  Extra files: `jwt.js`, `refreshToken.js`, `password.js`.
- **`user`** — user documents and lookups. No handlers of its own; it exists so
  other modules (auth, rbac, admin, and every service that resolves a display
  name) share one user access path.
- **`group`** — groups and membership: create, rename, delete, list, member
  add/remove/role-change, leave. Also owns `purgeGroupData`, the cascade used
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
  Extra file: `client.js`.

Supporting modules: `config.js`, `logging.js`, `errors.js`, `db/` (`index.js`
— connection, indexes, migrations; `model.js` — typed decoding of stored
documents), `server/` (HTTP server, router, route table, extractors), `serde/`
(request-body and query-string parsing), `rust.js` (string and date semantics
carried over from the Rust implementation), and `utils.js` (`insertWithId`,
`isDuplicateKey`, `parseObjectId`, regex escaping for user-supplied search
terms, and a Levenshtein distance used for typo-tolerant title search).

## Request lifecycle

```
HTTP request
  → CORS (preflights are answered here)
  → route match (server/routes.js; path matched after percent-decoding)
  → extractors: authenticatedUser | groupScoped | systemAdminUser,
                path, query, json body
  → handler        (parse + format validation)
  → service        (business rules + RBAC)
  → repository     (Mongo query, always group-scoped)
  → response, or ApiError → JSON error body
```

### Extractors (`server/middleware.js`, `server/extract.js`)

Authentication is not a global middleware but per-handler extractors, so each
route declares its own requirement: `handler([groupScoped, path, json(T)], fn)`.

- **`authenticatedUser`** — decodes the bearer token and yields `userId`.
  Entirely stateless: signature and `exp` only, no database lookup. That is safe
  because access tokens live 15 minutes; revocation is the refresh token's job.
  Used where authentication alone is the requirement.
- **`groupScoped`** — the tenant-scoped context. Decodes the token, parses the
  `{id}` path segment as the group id, and calls `RbacService.requireMember`.
  Membership is therefore re-checked on every request, so a user removed from a
  group is rejected on their next call rather than at token expiry. Handlers
  scoped to a group take this and never parse a group id themselves.
- **`systemAdminUser`** — decodes the token and requires the global System Admin
  role. Used by every `/admin` route.

The dispatcher runs a handler's extractors in three passes — synchronous checks
(token, ids, content type, query string) in argument order, then the JSON body,
then the database-backed checks (membership, admin role) — which decides which
error a request with several problems gets.

The extractor is the request-level half of RBAC only. Service-layer checks still
run underneath it — both layers are required.

### RBAC (`rbac/service.js`)

`RbacService` answers one question: what is this user's relationship to this
group, or to the system?

| Method | Rule |
| --- | --- |
| `requireMember` | Returns the membership (with role) or `Forbidden`. |
| `requireGroupAdmin` | Member *and* role is Group Admin. |
| `requireSystemAdmin` | Global role is System Admin. |
| `requireOwnerOrGroupAdmin` | Pure function: Group Admin, or the resource's creator. |

Two deliberate absences: **group isolation** is not enforced here — it is
enforced by every repository query filtering on `group_id` — and the
**sole-Group-Admin succession guard** stays in `GroupService`, since it is
membership business logic rather than a reusable primitive.

Non-membership returns `Forbidden`, never `NotFound`, so a non-member cannot
probe whether a group id exists. The same reasoning applies to a missing user in
`requireSystemAdmin`.

### Errors (`errors.js`)

One class, `ApiError`, is what every handler and service throws. Every error
serializes to the same body:

```json
{ "error": { "code": "forbidden", "message": "..." } }
```

| Kind | Status | Notes |
| --- | --- | --- |
| `InvalidCredentials` | 401 | |
| `Unauthenticated` | 401 | Missing/invalid/expired token. |
| `Forbidden` | 403 | Message is deliberately generic. |
| `NotFound` | 404 | |
| `DuplicateEmail` | 409 | |
| `Conflict` | 409 | Caller-supplied message. |
| `Validation` | 400 | Caller-supplied message. |
| `RateLimited` | 429 | The caller hit *our* per-user AI chat quota. |
| `AiUnavailable` | 503 | Upstream AI provider returned 429 or 5xx. Transient. |
| `Internal` | 500 | |

Repository and driver errors never reach the client directly: anything thrown
that isn't an `ApiError` (a MongoDB failure, a malformed stored document, a
bcrypt failure) becomes `Internal`, while the user, group and link repositories
map their duplicate-key cases to `DuplicateEmail` / `Conflict`.

Malformed requests that never reach a handler are answered as plain text, not
JSON: an unparsable or mistyped JSON body (`Json deserialize error: …`), a wrong
`Content-Type` (`Content type error`), a body over 2 MiB (413), or a bad query
string (`Query deserialize error: …`).

## Authentication

Two tokens, with different jobs.

**Access token** — a JWT (HS256; `sub` = user id, `exp`), 15-minute TTL, sent by
the client as `Authorization: Bearer`. Verified statelessly, with 60 seconds of
leeway on `exp`.

**Refresh token** — 32 bytes of CSPRNG output, hex-encoded. Only its SHA-256
hash is stored, so a leaked database cannot be used to mint sessions; SHA-256 is
adequate (rather than bcrypt) precisely because the input is already
high-entropy. TTL is 30 days, enforced both by the stored `expires_at` and by a
MongoDB TTL index that reaps spent tokens without a cron job.

It travels in a cookie scoped to `path=/api/v1/auth`, so it is never sent on
unrelated API calls. `httpOnly` keeps it away from JS. `Secure` is
config-driven because browsers refuse Secure cookies over plain HTTP, and
`SameSite` follows it: `None` when secure (production deploys frontend and API
on unrelated sites, and browsers honor `None` only alongside `Secure`), `Strict`
for local HTTP development, where a different port on localhost is still
same-site.

Passwords are bcrypt (cost 12, `$2b$` hashes).

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
| `/auth` | mixed | `register`, `login` (none); `me`, `me` PATCH, `me/password` (`authenticatedUser`); `refresh`, `logout` (cookie only). |
| `/groups` | `authenticatedUser` for create/list; `groupScoped` for everything under `/{id}` | Group CRUD, membership, and all nested resources: `tickets`, `comments`, `reactions`, `activity`, `links`, `references`. |
| `/ai` | `groupScoped` (`/ai/groups/{id}/tickets/{ticket_id}/…`) | `summarize`, `analyze`, and conversation create/list/delete plus message send/list. |
| `/admin` | `systemAdminUser` | `users`, `groups`, `groups/{id}` DELETE, `audit-log`, and `users/{id}/{deletion-check,delete,promote,demote}`. |

Every group-scoped path is `/groups/{id}/...`. There is no "active group" on the
server: the group is always named explicitly in the path. Unmatched paths and
methods (including `HEAD`) get an empty 404.

## Cross-cutting behaviors

**Group isolation.** Every repository method takes `group_id` and puts it in the
query filter. A ticket id from another group therefore reads as `NotFound`
rather than leaking that it exists elsewhere. Services that receive both a group
and a nested resource id (comments, activity, links, AI) re-verify that the
ticket belongs to the group, because `groupScoped` only proves membership in the
group — not that the ticket is in it.

**Ticket numbering.** Each group has its own `ticket_number` sequence, allocated
by an atomic counter (`TicketRepository.nextTicketNumber`) and backed by a
unique index on `(group_id, ticket_number)`.

**Activity logging.** Ticket updates diff before/after values and record an event
only when something actually changed. Entries carry the actor id; the display
name is resolved at read time so later renames are reflected.

**Cascade deletes.** Deleting a ticket clears its comments, reactions, activity,
links (both directions, so no dangling relation is left on the other end),
references, and AI insights/conversations. Deleting a group runs
`purgeGroupData` (`group/service.js`) over the same surface for every ticket in
it.

**Uniqueness.** Application-level pre-checks (duplicate email, duplicate member,
duplicate link) exist to produce good error messages; the unique index is what
actually rejects a concurrent duplicate. `utils.isDuplicateKey` recognizes error
code 11000.

## AI module

`AiService` takes its provider as a constructor argument, with `GeminiClient` as
the production one (`AiService.create`), so the service can be exercised against
a fake without network access. Model is `gemini-3.5-flash`, **pinned**. It was
previously the alias `gemini-flash-latest`, which drifted onto
`gemini-3.7-flash` — free tier 5 requests/minute, plus frequent
`503 UNAVAILABLE` under load. Measured ~75% of calls failing, and every one
surfaced to the client as an opaque 500. Pinning is what stops an alias changing
the quota profile without a code change; re-pin deliberately and re-check the
model's rate limit when upgrading. `analyze` requests a JSON response schema so
the output is parsed rather than scraped from prose.

Provider failures are mapped rather than collapsed: upstream `429`/`5xx` become
`AiUnavailable` (503, "temporarily unavailable — please try again in a moment"),
anything else stays `Internal`. The upstream status and body are written to the
log before being discarded, since they are never safe to return to the client.
Note this is a genuinely transient class of failure with **no retry** in the
client yet — a single 429 still fails the request.

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

## Compatibility layer

These modules exist so the server answers exactly as the Rust implementation
did, including for malformed input, rather than with a framework's defaults:

- **`serde/`** — parses JSON bodies and query strings into the request types in
  each `models.js`, with the original parser's rules and error texts (e.g.
  ``missing field `email` at line 1 column 2``), so clients that display server
  messages see the same thing.
- **`server/router.js`** — route matching on the percent-decoded path, exact and
  case-sensitive, one method per route, tried in table order.
- **`server/mime.js`, `server/cookies.js`** — which `Content-Type` values count
  as JSON, and how the `Cookie` header is split and decoded.
- **`rust.js`** — validation semantics that differ between the languages:
  whitespace for `trim` (U+0085 yes, U+FEFF no), UTF-8 byte length for the
  password and ticket-title limits, code-point ordering for reactions, and the
  RFC 3339 timestamp format (fractional seconds only when non-zero).
- **`db/model.js`** — stored documents are decoded strictly, so a document with
  a missing or wrongly typed field fails the request with 500 rather than
  leaking a partial shape. This is why new non-optional fields need a backfill.

## Tests

There are currently **no automated tests in the repository**. The translation
was verified by running both implementations side by side against identical
requests and comparing responses, outgoing Gemini requests, and the resulting
database contents; that harness was not committed.
