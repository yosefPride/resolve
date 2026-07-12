# Token System

## Overview

This app uses two tokens per session:

- **Access token** — a short-lived JWT that authorizes API requests. It is stateless: the server verifies it by signature and expiry alone, with no database lookup.
- **Refresh token** — a long-lived opaque token, persisted (hashed) in MongoDB, used only to mint new access tokens via `POST /api/v1/auth/refresh`.

They exist together to balance two conflicting needs: requests need to be authorized cheaply and often (no DB round-trip per request), but sessions also need to be revocable (logout, rotation) — which a purely stateless token can't support on its own. The access token handles the first need; the refresh token, backed by a DB row, handles the second. Splitting them means a stolen access token is only useful for a few minutes, while the durable, revocable session state (the refresh token) never leaves an httpOnly cookie.

## Access Token

- **Type**: JWT (`jsonwebtoken` crate), signed with `HS256` (via `Header::default()`).
- **Claims** (`src/auth/claims.rs`): `sub` (user ID as a string) and `exp` (Unix timestamp). No other claims are included — no roles, group memberships, or scopes are embedded in the token.
- **Generated**: `jwt::issue_token` in `src/auth/jwt.rs:12`, called from `AuthService::issue_session` (`src/auth/service.rs:32`), which is shared by register, login, and refresh.
- **Lifetime**: 15 minutes (`ACCESS_TOKEN_TTL_MINUTES` in `src/auth/jwt.rs:10`).
- **Returned to client**: in the JSON response body as `jwt`, on register (`AuthResponse.jwt`), login (`AuthResponse.jwt`), and refresh (`RefreshResponse.jwt`). It is never set as a cookie.
- **Client usage**: the frontend (`frontend/src/lib/axios.js`) keeps the access token in a module-level JS variable (in memory only, not localStorage/sessionStorage) and attaches it as `Authorization: Bearer <token>` on every outgoing request via an Axios request interceptor.
- **Backend validation**: `jwt::decode_token` (`src/auth/jwt.rs:35`) checks the signature and expiry using the `jsonwebtoken` crate's default `Validation`. There is no database lookup, no revocation check, and no group/role check at this layer — validation is purely "was this signed by us and not yet expired."
- **Responsible component**: the `AuthenticatedUser` extractor (`src/server/middleware.rs`), which implements Actix-web's `FromRequest`. Any handler that takes `AuthenticatedUser` as a parameter (e.g. `me` in `src/auth/handlers.rs:104`) gets this validation for free; handlers that don't take it (register, login, refresh, logout) skip it entirely.

## Refresh Token

- **Generated**: `refresh_token::generate` (`src/auth/refresh_token.rs:15`) creates 32 bytes (256 bits) of CSPRNG output, hex-encodes it as the raw token, and SHA-256-hashes it to get the stored hash. Returns `(raw, hash)`.
- **Raw token returned to client?** Yes, but only as an httpOnly cookie — never in a JSON response body. The raw value is set via `Set-Cookie` in `auth_handlers::refresh_cookie` (`src/auth/handlers.rs:24`).
- **Client storage**: browser-managed httpOnly cookie named `refresh_token`, scoped to path `/api/v1/auth` so it's not sent on unrelated API calls. `SameSite=Strict`; `Secure` is config-driven (`Config::cookie_secure`) to allow plain-HTTP local dev. Being httpOnly, it is inaccessible to JavaScript — the frontend never reads or stores it directly.
- **Backend storage**: MongoDB, in the `refresh_tokens` collection, one document per outstanding token (`RefreshTokenDoc` in `src/auth/models.rs:37`).
- **Hashed?** Yes — only the SHA-256 hash (`token_hash`) is stored, never the raw token. A leaked database therefore cannot be used to mint sessions. (Comment in `refresh_token.rs:6-9` notes SHA-256, not bcrypt, is intentional here: the token already has 256 bits of entropy, unlike a user-chosen password, so a slow hash isn't needed.)
- **Database model/collection**: `refresh_tokens` collection, fields: `_id`, `user_id` (`ObjectId`), `token_hash` (unique-indexed), `created_at`, `expires_at`, `revoked_at` (`Option`).
- **Lifetime**: 30 days (`REFRESH_TOKEN_TTL_DAYS` in `src/auth/refresh_token.rs:4`), fixed at issuance (`expires_at = now + 30 days`), matching the cookie's `max_age`.
- **Revocation behavior**: `revoked_at` is set (not deleted) on:
  - **Rotation** — the presented token is revoked the moment `/auth/refresh` succeeds (`AuthService::refresh`, `src/auth/service.rs:98`), before the new one is issued.
  - **Logout** — `AuthService::logout` (`src/auth/service.rs:106`) revokes the token matching the cookie presented to `/auth/logout`.
  - A revoked or expired token simply won't match `find_active_by_hash` (`src/auth/repository.rs:40`, which filters `revoked_at: null` and `expires_at: { $gt: now }`); there is no separate reuse-detection or alerting path — a replayed revoked/expired token is treated the same as any invalid token (`ApiError::Unauthenticated`).
  - Expired documents (regardless of `revoked_at`) are also physically deleted by a MongoDB TTL index on `expires_at` (`src/db.rs:42-53`) — no application-level cleanup job exists or is needed.

## Login Flow

```
User submits credentials (POST /auth/login)
↓
validate_login() checks email/password are non-empty (src/auth/handlers.rs:61)
↓
AuthService::login: user looked up by email, password verified with bcrypt (src/auth/service.rs:68)
↓
AuthService::issue_session called:
  - Access token (JWT) created via jwt::issue_token, 15 min expiry
  - Refresh token created via refresh_token::generate (raw + SHA-256 hash)
  - Refresh token hash inserted into `refresh_tokens` collection (AuthRepository::insert)
↓
Handler returns HTTP 200 with:
  - JSON body: { user, jwt }
  - Set-Cookie: refresh_token=<raw> (httpOnly, Secure*, SameSite=Strict, 30-day max-age)
```

`register` follows the identical sequence, with a user-creation step (bcrypt password hash + insert) before `issue_session`.

## Normal Request Flow

```
Client request to a protected route (e.g. GET /auth/me)
↓
Authorization: Bearer <jwt> header (attached by frontend Axios interceptor)
↓
AuthenticatedUser::from_request (src/server/middleware.rs) runs as an Actix extractor
  - Requires the "Bearer " prefix; missing/malformed header → 401 (ApiError::Unauthenticated)
↓
jwt::decode_token verifies signature + exp (no DB call)
  - Invalid signature or expired → 401
  - claims.sub parsed into a Mongo ObjectId → 401 if malformed
↓
Request proceeds to the handler with AuthenticatedUser { user_id } injected
```

Note: this is opt-in per handler — only handlers that declare `AuthenticatedUser` as a parameter get this check. `register`, `login`, `refresh`, and `logout` do not.

## Refresh Flow

```
Access token expires (after 15 min) — a subsequent request gets 401 from the API
↓
Frontend Axios response interceptor (frontend/src/lib/axios.js) catches the 401
  and (if not already retried, and not a login/register/refresh call itself)
  calls POST /auth/refresh
↓
refresh handler (src/auth/handlers.rs:120) reads the `refresh_token` cookie
  - No cookie → 401 (ApiError::Unauthenticated), no DB call
↓
AuthService::refresh (src/auth/service.rs:89):
  - Hashes the raw cookie value with SHA-256
  - find_active_by_hash: looks up a row with matching hash, revoked_at == null,
    expires_at > now — no match → 401
↓
Old token handling: the matched row is revoked immediately (revoke_by_id),
  BEFORE the new session is issued — single-use rotation
↓
New access token issued (issue_session): fresh JWT, 15 min expiry
↓
New refresh token issued (issue_session): fresh raw+hash pair, new DB row,
  30-day expiry from now — rotation is unconditional, every refresh call
  gets a brand new refresh token
↓
Handler returns { jwt } in the body and sets the new refresh_token cookie
```

The original request that triggered the 401 is then retried once by the frontend with the new access token. Concurrent 401s share a single in-flight refresh (both frontend `refreshPromise` in `axios.js` and the backend's single-use rotation guard against a second, now-stale cookie being replayed).

## Refresh Token Rotation

- **Implemented?** Yes, unconditionally, on every successful `/auth/refresh` call.
- **Old token handling**: revoked (`revoked_at` set), not deleted immediately — it remains in the collection until its original `expires_at` is reached, at which point MongoDB's TTL index removes it.
- **New token creation**: a brand-new raw token + hash pair via `refresh_token::generate`, inserted as a new document — not an update of the old row.
- **Expiration**: fixed, not sliding. Each new refresh token gets `expires_at = now + 30 days` at the moment it's issued; there is no mechanism that extends an existing token's expiry. A user who refreshes daily still gets a session that dies 30 days after their *last* refresh (since each refresh mints a fresh 30-day token) — practically sliding in effect, but implemented as a fixed TTL per token, not as an update to one persistent expiry.

## Logout Flow

```
POST /auth/logout (no access token required)
↓
Handler reads the `refresh_token` cookie, if present (src/auth/handlers.rs:142)
↓
AuthService::logout: hashes the raw token, revokes the matching row
  (revoke_by_hash — matches on hash alone, not on revoked_at/expires_at state)
↓
Response: 200 OK, Set-Cookie clears refresh_token (max-age 0)
```

- **Refresh token revoked or deleted?** Revoked (`revoked_at` set), not deleted. The row persists until TTL expiry.
- **Scope**: per-device/per-session — only the refresh token in the request's cookie is revoked, so other logged-in devices/sessions for the same user are unaffected.
- **Existing access tokens**: remain valid until their own expiry (≤15 minutes). Logout does not — and cannot, given stateless verification — invalidate an already-issued access token early. A logged-out user can still make authorized requests with a still-live access token until it naturally expires.
- **Missing/unknown cookie**: treated as a no-op success, not an error — the end state ("this token no longer works") already holds.

## Security Design

- **Why refresh tokens are hashed**: the refresh token is a long-lived bearer credential capable of minting new sessions for 30 days. Storing only its SHA-256 hash means a database compromise (backup leak, injection, insider access) doesn't hand out usable session tokens — the attacker would need to reverse a 256-bit random value, which SHA-256 makes infeasible even though it's a fast hash. (Fast hashing is acceptable here — unlike passwords — because the input already has full cryptographic entropy, not user-guessable structure.)
- **Why access tokens are short-lived**: because they're validated statelessly (signature + exp only, no DB check), there is no way to revoke one early. A 15-minute TTL bounds the damage window of a stolen access token to that duration, without requiring every single API request to hit the database.
- **Why refresh tokens exist at all**: they move revocability into a token type that's rarely transmitted (only to `/auth/refresh`, never on ordinary API calls) and never exposed to JavaScript (httpOnly cookie), while letting ordinary requests stay fast and stateless via the access token.
- **How token theft risk is reduced**:
  - Access token: short TTL limits exposure; never stored in a cookie so it isn't subject to CSRF (it's attached manually via an `Authorization` header the browser won't send automatically cross-site).
  - Refresh token: httpOnly (unreadable by XSS'd JS), `Secure` in production, `SameSite=Strict` (not sent on cross-site requests, mitigating CSRF), scoped to the `/api/v1/auth` path (not leaked to unrelated endpoints), hashed at rest, and single-use via rotation — a copied/replayed token stops working the moment the legitimate client refreshes.

## Configuration Summary

| Token | Lifetime | Storage | Rotation |
|------|----------|---------|----------|
| Access Token | 15 minutes (fixed) | In-memory JS variable on client; not persisted anywhere on the backend | N/A — reissued fresh on every login/refresh, never renewed in place |
| Refresh Token | 30 days (fixed per token) | httpOnly cookie (raw value) on client; SHA-256 hash in MongoDB `refresh_tokens` collection on backend | Yes — single-use, rotated unconditionally on every `/auth/refresh` call; old token revoked, new token+row created |
