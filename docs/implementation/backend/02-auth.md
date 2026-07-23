# Backend — Auth & Users

Covers: `src/auth/` (8 files) and `src/user/` (3 files).

`user/` is the plain persistence layer for the `users` collection. `auth/` owns everything
about *proving* identity: passwords, tokens, sessions. The split point is the password
hash — `UserService` stores and returns it, `AuthService` is the only thing that verifies it.

---

# `src/user/`

## `user/models.rs` (47 lines)

### `enum GlobalRole { SystemAdmin }`
Single variant. **No `#[serde(rename_all)]`**, so it serializes as the string `"SystemAdmin"`
— which is why the frontend's `utils/roles.js` checks `user.global_role === 'SystemAdmin'`
in PascalCase, unlike the group `Role` enum which is snake_case. Easy detail to get wrong.

### `struct User`
The stored document: `id` (`_id`, `Option<ObjectId>`, skipped when `None` so Mongo
generates it), `email`, `password_hash`, `name`, `global_role: Option<GlobalRole>`,
`created_at: BsonDateTime`.

`global_role` is `Option` and is set to `None` by `UserRepository::create` — **there is no
code path that promotes a user to System Admin.** It has to be set directly in the database.

### `struct CreateUserInput { email, name, password_hash }`
Input DTO. Note it takes an already-hashed password — hashing is the caller's job
(`AuthService`), which keeps bcrypt out of the user module entirely.

### `struct UserResponse`
The client-facing shape: `id` as a hex `String`, `email`, `name`, `global_role`,
`created_at` as `chrono::DateTime<Utc>` (serializes to RFC3339). **No `password_hash`** —
that's the whole point of the type.

### `impl From<User> for UserResponse`
The conversion. Two defensive details: `id.map(to_hex).unwrap_or_default()` yields `""` for
an unsaved user, and `DateTime::from_timestamp_millis(...).unwrap_or_default()` yields the
epoch for an out-of-range timestamp. Neither should happen; neither panics.

## `user/repository.rs` (138 lines)

### `enum UserRepoError { DuplicateEmail, Database(mongodb::error::Error) }`
With `Display`, `std::error::Error`, and `From<mongodb::error::Error>`.

### `fn is_duplicate_key(err) -> bool`
Checks for Mongo error code `11000`, and — importantly — checks **two different error
shapes**: `ErrorKind::Write(WriteFailure::WriteError)` for `insert_one`, and
`ErrorKind::Command` for `find_one_and_update` (because `findAndModify` is a command, not a
plain write). Missing the second would let a duplicate email on profile update surface as a
500 instead of a 409.

### `struct UserRepository { collection: Collection<User> }`
Typed collection over `"users"`.

| Method | What it does |
|---|---|
| `new(db)` | `db.collection("users")` |
| `create(input)` | Builds a `User` with `global_role: None`, `created_at: now`, inserts, and returns the struct with the generated `_id` filled in (avoiding a read-after-write). |
| `update_profile(id, name, email)` | `find_one_and_update` with `$set` on name+email, `ReturnDocument::After` so it returns the updated doc in one round trip. Returns `Option<User>` — `None` means no such id. |
| `update_password_hash(id, hash)` | `update_one` + `$set`. Returns `bool` from `matched_count > 0`. |
| `find_by_email(email)` | Exact match — **case-sensitive**, which is why the member-lookup endpoint requires an exact email. |
| `find_by_id(id)` | By `_id`. |
| `list_all(search: Option<&str>)` | Empty filter when `search` is absent/blank; otherwise `{$or: [{name: rx}, {email: rx}]}` using `substring_regex`. Collects the whole cursor — no pagination. |
| `delete(id)` | `delete_one`, returns `deleted_count > 0`. |

## `user/service.rs` (71 lines)

A thin pass-through over the repository whose real job is **controlling which shape escapes**.

- `create`, `find_by_id`, `list_all`, `update_profile` → map to `UserResponse` (no hash).
- `find_by_email` and `find_full_by_id` → return the **full `User`, including `password_hash`**. Both carry an explicit comment saying this is intentional and who needs it (`find_by_email` for login, `find_full_by_id` for verifying the current password before an email change).
- `update_password_hash`, `delete` → pass through booleans.

If you're explaining this module, that two-tier accessor pattern is the thing worth pointing
at: the hash is reachable, but only through a differently-named method you have to
deliberately choose.

---

# `src/auth/`

## `auth/claims.rs` (7 lines)

### `struct Claims { sub: String, exp: usize }`
The entire JWT payload: subject (the user's ObjectId as hex) and expiry. **No role, no group,
no email.** That's the design — the token proves *who*, never *what they may do*, so a
demoted or removed user loses access on their next request rather than at token expiry.

## `auth/jwt.rs` (68 lines)

### `const ACCESS_TOKEN_TTL_MINUTES: i64 = 15`
The reason it's short: tokens are verified statelessly, so there's no revocation check; the
exposure window of a stolen token is bounded by this constant alone.

### `issue_token(user_id, secret) -> Result<String, jsonwebtoken::errors::Error>`
Computes `exp = now + 15min` and delegates to `issue_token_with_exp`.

### `issue_token_with_exp(user_id, secret, exp)`
`jsonwebtoken::encode` with default header (HS256) and `EncodingKey::from_secret`. Exposed
separately so tests can mint already-expired tokens.

### `decode_token(token, secret) -> Result<Claims, _>`
`jsonwebtoken::decode` with `Validation::default()` — which checks the signature **and
validates `exp`** automatically. Returns just the claims.

Three inline unit tests: round-trip, wrong secret rejected, expired token rejected.

## `auth/password.rs` (17 lines)

### `const WORK_FACTOR: u32 = 12`
Explicit rather than `DEFAULT_COST`. The comment explains cost is the exponent in 2^cost
iterations.

### `hash_password(password) -> Result<String, BcryptError>` / `verify_password(password, hash) -> Result<bool, BcryptError>`
Thin wrappers over `bcrypt::hash` / `bcrypt::verify`. Note `verify_password` returns
`Result<bool>` — callers must check the `bool`, not just that it didn't error.

## `auth/refresh_token.rs` (53 lines)

### Constants
- `REFRESH_TOKEN_TTL_DAYS: i64 = 30`
- `REFRESH_TOKEN_COOKIE: &str = "refresh_token"`
- `TOKEN_BYTES: usize = 32` — 256 bits of CSPRNG output.

### `generate() -> (String, String)`
Fills 32 bytes from `rand::rng()`, hex-encodes to get `raw`, then returns `(raw, hash_token(&raw))`.
The contract: **`raw` goes to the client and is never stored; `hash` is persisted.** A leaked
database therefore can't be used to mint sessions.

### `hash_token(raw: &str) -> String`
SHA-256, hex-encoded. The comment explains why a fast hash is fine here and bcrypt isn't
needed: unlike a password, the token already has 256 bits of entropy, so there's nothing to
brute-force.

### `encode_hex(bytes: &[u8]) -> String` (private)
`format!("{b:02x}")` per byte.

Two inline tests: distinct tokens with matching hashes, and hash determinism.

## `auth/models.rs` (62 lines)

Request/response DTOs plus one stored document.

| Type | Role |
|---|---|
| `RegisterRequest { email, password, name }` | Body of `POST /auth/register`. |
| `LoginRequest { email, password }` | Body of `POST /auth/login`. |
| `UpdateMeRequest { name: Option, email: Option, current_password: Option }` | Body of `PATCH /auth/me`. All optional so a client can update either field alone; `current_password` is only demanded when the email actually changes. |
| `ChangePasswordRequest { current_password, new_password }` | Body of `POST /auth/me/password`. |
| `AuthResponse { user: UserResponse, jwt: String }` | Response of register/login. |
| `RefreshResponse { jwt: String }` | Response of refresh — **deliberately only the token**; the rotated refresh token travels as a cookie, never in a body. |
| `RefreshTokenDoc` | The `refresh_tokens` document: `id`, `user_id`, `token_hash`, `created_at`, `expires_at`, `revoked_at: Option<BsonDateTime>`. |

## `auth/repository.rs` (94 lines)

### `struct AuthRepository { collection: Collection<RefreshTokenDoc> }`
Over `"refresh_tokens"`. All methods return `Result<_, mongodb::error::Error>` directly —
no custom error enum, since there's no domain-specific failure to distinguish.

| Method | What it does |
|---|---|
| `insert(user_id, token_hash, expires_at)` | Inserts a row with `created_at: now`, `revoked_at: None`. |
| `find_active_by_hash(token_hash)` | **The important one.** Filter is `{token_hash, revoked_at: null, expires_at: {$gt: now}}`. Because "active" is baked into the query, a replayed (already-rotated) token or an expired one simply isn't found — no separate reuse-detection branch exists anywhere. |
| `revoke_by_id(id)` | `$set: {revoked_at: now}` by `_id`. Used by rotation. |
| `revoke_by_hash(token_hash)` | Same, matched by hash. Used by logout. |
| `revoke_all_for_user_except(user_id, except_hash: Option<&str>)` | `update_many` over `{user_id, revoked_at: null}`, adding `{token_hash: {$ne: hash}}` when a hash is given. `None` revokes **all** of them. This is the password-change behavior: sign out every other device, keep the current one. |

## `auth/service.rs` (182 lines)

### `struct AuthService { user_service, auth_repo, jwt_secret }`
Constructed per request via `AuthService::new(&state.db, state.config.jwt_secret.clone())`.

### `async fn issue_session(&self, user_id: &str) -> Result<(String, String), ApiError>` (private)
The shared session-minting routine used by **register, login, and refresh**, so all three
produce identical session state:
1. `jwt::issue_token`.
2. `refresh_token::generate()` → `(raw, hash)`.
3. `expires_at = now + 30 days`.
4. `auth_repo.insert(user_object_id, hash, expires_at)`.
5. Returns `(jwt, raw_refresh_token)`.

### `async fn register(input) -> Result<(AuthResponse, String), ApiError>`
Hash password → `user_service.create` → `issue_session`. Returns the JSON body **and** the
raw refresh token separately, because the handler (not the service) is responsible for
turning it into a cookie. That separation keeps HTTP concerns out of the service layer.

### `async fn login(input) -> Result<(AuthResponse, String), ApiError>`
`find_by_email` → `ok_or(InvalidCredentials)` → `verify_password` → `if !valid { return
InvalidCredentials }` → convert to `UserResponse` → `issue_session`.
Both failure paths return the same error, so the endpoint doesn't reveal which emails exist.

### `async fn update_me(user_id, input) -> Result<UserResponse, ApiError>`
1. `find_full_by_id` (needs the hash) → `ok_or(Unauthenticated)`.
2. Falls back to the existing name/email for any field the client omitted.
3. **Only if the email actually differs**: requires `current_password` (else `Validation`), verifies it (else `InvalidCredentials`).
4. `user_service.update_profile(id, name.trim(), email.trim())`.

The comment explains why this lives in `AuthService` rather than `UserService`: it needs the
password check.

### `async fn change_password(user_id, input, current_token_hash: Option<&str>) -> Result<(), ApiError>`
1. Load full user, verify `current_password` → else `InvalidCredentials`.
2. Hash the new password, `update_password_hash`.
3. `auth_repo.revoke_all_for_user_except(user_id, current_token_hash)`.

The `current_token_hash` parameter is what makes "sign out other devices but not this one"
work; the handler supplies it from the request's own cookie.

### `async fn refresh(raw_refresh_token) -> Result<(String, String), ApiError>`
1. Hash the presented token.
2. `find_active_by_hash` → `ok_or(Unauthenticated)`.
3. `revoke_by_id` **first** (single-use rotation), then
4. `issue_session` for `record.user_id`.

Revoking before issuing is the ordering that makes a stolen copy stop working the moment the
legitimate client refreshes.

### `async fn logout(raw_refresh_token) -> Result<(), ApiError>`
`revoke_by_hash`. A missing/unknown token is a **no-op, not an error** — the desired end
state ("this token no longer works") already holds.

## `auth/handlers.rs` (224 lines)

### Cookie helpers

#### `fn refresh_cookie(raw_token, secure) -> Cookie<'static>`
Attributes: `path("/api/v1/auth")` (so the token is never sent on unrelated API calls),
`http_only(true)` (out of JS reach), `secure(config-driven)`, `same_site(Strict)`,
`max_age(30 days)`.

The long comment explains why `SameSite=Strict` is hardcoded rather than configurable:
"site" ignores port and (for same-registrable-domain hosts) subdomain, so it already covers
both intended topologies — dev frontend on a different port, prod frontend/API on different
subdomains. It would only need relaxing if the two ended up on unrelated domains.

#### `fn expired_refresh_cookie(secure) -> Cookie<'static>`
Same attributes, empty value, `max_age(ZERO)` — instructs the browser to drop it.

### Validators (pure, run before any service call)

- `validate_register` — email non-blank and contains `@`; password ≥ 8; name non-blank.
- `validate_login` — email and password non-empty.
- `validate_update_me` — at least one of name/email present; if email present it must be non-blank and contain `@`; if name present it must be non-blank.
- `validate_change_password` — `current_password` non-empty; `new_password` ≥ 8.

### Handlers

| Handler | Extractors | Flow |
|---|---|---|
| `register` | `state`, `Json<RegisterRequest>` | validate → `AuthService::register` → `201` + `{user, jwt}` + refresh cookie |
| `login` | `state`, `Json<LoginRequest>` | validate → `AuthService::login` → `200` + body + cookie |
| `me` | **`AuthenticatedUser`**, `state` | `UserService::find_by_id` → `200 UserResponse`, or `Unauthenticated` if the user was deleted while holding a valid token |
| `update_me` | `AuthenticatedUser`, `state`, `Json` | validate → `AuthService::update_me` → `200` |
| `change_password` | `AuthenticatedUser`, **`HttpRequest`**, `state`, `Json` | validate → reads its own refresh cookie and hashes it → passes that as `current_token_hash` → `200`, empty body |
| `refresh` | **`HttpRequest`**, `state` — *no `AuthenticatedUser`* | cookie → `ok_or(Unauthenticated)` → `AuthService::refresh` → `200 {jwt}` + rotated cookie |
| `logout` | `HttpRequest`, `state` — *no `AuthenticatedUser`* | if a cookie exists, revoke it; always `200` + expired cookie |

The two endpoints without `AuthenticatedUser` are the ones to call out when explaining this:
`refresh` can't require a valid access token (the whole reason you're refreshing is that
yours expired), and `logout` shouldn't (an already-invalid session should still be able to
clear its cookie).
