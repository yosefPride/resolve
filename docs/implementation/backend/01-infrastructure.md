# Backend — Infrastructure & Cross-Cutting

Covers: `main.rs`, `lib.rs`, `config.rs`, `db.rs`, `state.rs`, `server/routes.rs`,
`errors/`, `utils/`.

---

## `src/main.rs` (66 lines)

The binary entry point. Declares the full module tree (`admin, ai, auth, comment, config,
db, errors, group, rbac, server, state, ticket, user, utils`) and contains exactly one
function.

### `async fn main() -> std::io::Result<()>`
Annotated `#[actix_web::main]`, which wraps the async body in a Tokio runtime.

Sequence:
1. `Config::from_env()` — maps a missing env var to `io::ErrorKind::InvalidInput` so the process fails loudly at boot rather than at first request.
2. `db::connect(&config)` — any failure becomes `io::Error::other`.
3. `db::database(&client, &config)` then `db::ensure_indexes(&database)`.
4. Builds `web::Data<AppState>` **once, outside the server closure**, so all worker threads share one `Arc` (one connection pool, one config).
5. `HttpServer::new(closure)` — the closure runs per worker thread. Inside it:
   - **CORS**: `allowed_origin(&config.frontend_origin)` (explicit, not permissive), methods `GET/POST/PATCH/DELETE`, headers `Authorization` + `Content-Type`, `supports_credentials()`, `max_age(3600)`. The comment explains why: the refresh cookie needs `credentials: 'include'`, and the CORS spec forbids combining credentialed requests with a wildcard origin. So a wildcard is not an option here.
   - `Logger::default()`.
   - `web::scope("/api/v1").configure(server::routes::configure)`.
6. `.bind(bind_address)?.run().await`.

Note: `PUT` is absent from the allowed methods — the API only ever uses `PATCH` for updates.

---

## `src/lib.rs` (11 lines)

Re-exports a **subset** of the module tree as a library crate: `admin, auth, config, errors,
group, rbac, server, state, ticket, user, utils`.

Deliberately missing: `ai`, `comment` (both empty), and `db` (tests build their own client).
This is the crate the integration tests in `backend/tests/` import as `resolve::...`.

---

## `src/config.rs` (33 lines)

### `struct Config`
Four fields, all resolved at boot:
- `mongo_uri: String` — required.
- `jwt_secret: String` — required.
- `cookie_secure: bool` — defaults `true`; only the literal string `"false"` turns it off. Needed because a browser silently refuses to store a `Secure` cookie over plain HTTP, which local dev uses.
- `frontend_origin: String` — defaults `"http://localhost:5173"` (Vite's default port).

### `Config::from_env() -> Result<Self, std::env::VarError>`
Calls `dotenvy::dotenv().ok()` (ignoring failure — in production the vars come from the real
environment, not a file), then reads each var. `?` on the two required ones propagates
`VarError`.

### `Config::bind_address(&self) -> String`
Returns the hardcoded `"127.0.0.1:8080"`. Not configurable — worth flagging if you ever
deploy this, since `127.0.0.1` won't accept external connections.

---

## `src/state.rs` (8 lines)

### `struct AppState { db: Database, config: Config }`
The entire shared application state. Handed to Actix as `web::Data<AppState>` (an `Arc`), so
every handler and extractor reaches it via `req.app_data::<web::Data<AppState>>()` or a
`state: web::Data<AppState>` parameter.

Note the `mongodb::Database` here is itself a cheap handle over an internally-`Arc`'d
connection pool — cloning it (which every `Repository::new` effectively does via
`db.collection(...)`) is nearly free. That's why services can be constructed per-request.

---

## `src/db.rs` (132 lines)

### `async fn connect(config: &Config) -> Result<Client, Error>`
`Client::with_uri_str` does **not** open a connection (the driver connects lazily), so this
follows it with `run_command(doc!{"ping": 1})` against the `resolve` database. That turns a
bad URI or unreachable cluster into a startup failure instead of a mysterious first-request
error. Prints a success line on connect.

### `fn database(client: &Client, _config: &Config) -> Database`
Returns `client.database("resolve")`. The database name is hardcoded; the `_config` param is
unused (a placeholder for making it configurable later).

### `async fn ensure_indexes(db: &Database) -> Result<(), Error>`
Creates every index the app relies on. Idempotent — Mongo's `createIndex` is a no-op if the
index already exists, so this runs safely on every boot. Full list with rationale:

| Collection | Keys | Options | Why |
|---|---|---|---|
| `users` | `email: 1` | unique | Makes duplicate registration impossible even if two requests race past the app-level check. |
| `refresh_tokens` | `token_hash: 1` | unique | One row per token. |
| `refresh_tokens` | `expires_at: 1` | TTL, `expireAfterSeconds: 0` | Mongo's background reaper deletes spent/expired tokens — no cron job needed. |
| `group_members` | `group_id: 1, user_id: 1` | unique | Enforces one membership row per (group, user), making `DuplicateMember` atomic instead of check-then-insert. Also serves every per-group role lookup. |
| `group_members` | `user_id: 1` | — | Serves "list my groups", which filters on `user_id` alone — the compound index can't help, since `user_id` isn't its prefix. |
| `admin_audit_log` | `group_id: 1` | — | Serves `GET /admin/audit-log?group_id=`. |
| `admin_audit_log` | `deleted_user_id: 1` | — | Serves `?user_id=`. Separate (not compound) because the filters are independent. |
| `tickets` | `group_id: 1` | — | Every ticket query filters on this. |
| `tickets` | `group_id: 1, status: 1` | — | Serves `count_open_by_group` and status filtering. |
| `tickets` | `group_id: 1, created_by: 1` | — | Serves the `creator` filter. |
| `tickets` | `group_id: 1, ticket_number: 1` | unique | Belt-and-braces on the per-group sequence, on top of the atomic counter. |

No indexes for `comments` or `ai_*` — those collections don't exist in code.

---

## `src/server/routes.rs` (75 lines)

### `fn configure(config: &mut web::ServiceConfig)`
The single source of truth for the API surface. Everything is already inside
`/api/v1` (applied in `main.rs`). Four scopes:

**`/auth`** — no extractor at the route level; each handler decides.
```
POST   /auth/register
POST   /auth/login
GET    /auth/me
PATCH  /auth/me
POST   /auth/me/password
POST   /auth/refresh
POST   /auth/logout
```

**`/groups`** — mixed: the collection routes use `AuthenticatedUser`, the `{id}` routes use `GroupScoped`.
```
POST   /groups
GET    /groups
GET    /groups/{id}
PATCH  /groups/{id}
DELETE /groups/{id}
GET    /groups/{id}/users
POST   /groups/{id}/users
GET    /groups/{id}/users/lookup
PATCH  /groups/{id}/users/{user_id}
DELETE /groups/{id}/users/{user_id}
POST   /groups/{id}/tickets
GET    /groups/{id}/tickets
GET    /groups/{id}/tickets/{ticket_id}
PATCH  /groups/{id}/tickets/{ticket_id}
DELETE /groups/{id}/tickets/{ticket_id}
```
Note the ticket routes are registered here, calling into `ticket_handlers` — they're nested
under the group scope rather than living in their own scope, which is what makes `{id}`
available to the `GroupScoped` extractor.

Route-ordering detail: `/{id}/users/lookup` is registered **before** `/{id}/users/{user_id}`,
so `lookup` isn't swallowed as a `user_id`.

**`/ai`** — `web::scope("/ai")` registered with **zero routes**. A placeholder.

**`/admin`** — all `SystemAdminUser`.
```
GET    /admin/users
GET    /admin/groups
DELETE /admin/groups/{id}
GET    /admin/audit-log
GET    /admin/users/{id}/deletion-check
POST   /admin/users/{id}/delete
```
The last two live in a nested `web::scope("/users/{id}")`.

---

## `src/errors/api_error.rs` (146 lines)

The error vocabulary shared by every layer.

### `enum ApiError`
Eight variants: `InvalidCredentials`, `Unauthenticated`, `Forbidden`, `NotFound`,
`DuplicateEmail`, `Conflict(String)`, `Validation(String)`, `Internal`.

### `ApiError::code(&self) -> &'static str`
Maps each variant to a stable machine-readable string (`"invalid_credentials"`,
`"forbidden"`, `"duplicate_email"`, `"validation_error"`, ...). The frontend branches on
these — e.g. `ProfileForm.jsx` checks for `duplicate_email`.

### `ApiError::message(&self) -> String`
Human-readable text. Two deliberate choices:
- `Forbidden` → `"you do not have permission to perform this action"` — does **not** distinguish "not a member" from "group doesn't exist", so a non-member can't enumerate group ids.
- `Conflict` and `Validation` carry their own message; everything else is a fixed string.

### `impl ResponseError for ApiError`
This is what lets handlers return `Result<HttpResponse, ApiError>` and have Actix render
the error automatically.
- `status_code()` — `InvalidCredentials`/`Unauthenticated` → 401, `Forbidden` → 403, `NotFound` → 404, `DuplicateEmail`/`Conflict` → 409, `Validation` → 400, `Internal` → 500.
- `error_response()` — serializes `{ "error": { "code", "message" } }`.

### The `From` impls
These are what make `?` work across layer boundaries. Every one of them **collapses database
detail into `ApiError::Internal`**, so raw Mongo errors are never exposed to a client:

| From | Behavior |
|---|---|
| `UserRepoError` | `DuplicateEmail` → `DuplicateEmail`; `Database(_)` → `Internal` |
| `GroupRepoError` | `DuplicateMember` → `Conflict("user is already a member of this group")`; `Database(_)` → `Internal` |
| `AdminRepoError` | always `Internal` |
| `TicketRepoError` | always `Internal` |
| `bcrypt::BcryptError` | `Internal` |
| `jsonwebtoken::errors::Error` | `Internal` |
| `mongodb::error::Error` | `Internal` |

---

## `src/utils/mod.rs` (77 lines)

Three pure functions, no I/O, all unit-tested inline.

### `escape_regex(input: &str) -> String`
Prefixes a backslash to each of `\ . + * ? ( ) | [ ] { } ^ $`. Without it, user input like
`a(b` would be an invalid regex (query error) or would let the caller inject pattern syntax
into a Mongo `$regex`.

### `substring_regex(term: &str) -> mongodb::bson::Regex`
Wraps `escape_regex(term)` with option `"i"`, producing a case-insensitive literal substring
matcher usable directly as a field filter: `doc! { "name": substring_regex(term) }`.
Used by `UserRepository::list_all` and `GroupRepository::list_all_groups` for admin search.

### `levenshtein_distance(a: &str, b: &str) -> usize`
Classic two-row dynamic-programming edit distance (`O(len_a × len_b)` time,
`O(len_b)` space — it keeps only `prev`/`curr` rows and swaps them). Operates on `Vec<char>`,
so it's correct for multi-byte UTF-8.

Not Mongo-native, so callers run it **in-process over an already-fetched, already
group-scoped result set** — see `TicketService::search_by_title`. That ordering matters: the
typo-tolerant search never widens the tenant boundary, because the boundary was applied by
the database query that produced the input.
