# Auth Module

JWT + bcrypt authentication with a short-lived access token backed by a
rotating refresh token. Group RBAC is not implemented yet — this module only
establishes *who* a request is from, not what they're allowed to do.

## Token model

Two tokens are issued on register/login:

- **Access token** — a JWT (`Claims { sub, exp }`), 15 minute TTL
  (`jwt::ACCESS_TOKEN_TTL_MINUTES`). Verified statelessly in
  `server::middleware::AuthenticatedUser`: signature + expiry only, no
  database lookup. This is safe *because* the TTL is short — a stolen access
  token is only useful for a few minutes, and revocation doesn't need to
  reach it before then.
- **Refresh token** — an opaque 256-bit random value, 30 day TTL
  (`refresh_token::REFRESH_TOKEN_TTL_DAYS`). Returned to the client only as an
  **httpOnly, SameSite=Strict cookie** scoped to `/api/v1/auth`
  (`refresh_token::REFRESH_TOKEN_COOKIE`), never in a JSON body and never
  readable by JS. The database stores only its SHA-256 hash
  (`refresh_token::hash_token`) — a leaked database can't be replayed into a
  session, the same principle as bcrypt for passwords, just with a fast hash
  since the token already has enough entropy.

  The `Secure` attribute is config-driven (`Config::cookie_secure`, env
  `COOKIE_SECURE`, default `true`) rather than hardcoded, since a real browser
  silently refuses to store a `Secure` cookie over plain HTTP — local dev
  needs `COOKIE_SECURE=false` unless it's served over HTTPS. `SameSite=Strict`
  itself is left fixed: "site" for SameSite purposes ignores port (and, for
  same-registrable-domain hosts, ignores subdomain), so Strict already works
  for a local frontend on a different port and for a production frontend/API
  split across subdomains of the same domain. It would only need to relax to
  `None` (+ `Secure`) if frontend and API ever ended up on genuinely unrelated
  domains.

  Consuming this cookie cross-origin also requires CORS to name the frontend
  origin explicitly (`Config::frontend_origin`, env `FRONTEND_ORIGIN`) with
  `supports_credentials()` — a wildcard origin cannot legally be combined with
  a credentialed (cookie-bearing) request. See `main.rs`.

There is no more `User.token_version` counter. Revocation now lives entirely
at the refresh-token layer (see below), not on the user document.

## Rotation policy

Every refresh token is single-use. `POST /auth/refresh`:

1. Hashes the presented token and looks up a matching row that is not
   revoked and not expired (`AuthRepository::find_active_by_hash`).
2. Revokes that row (`revoked_at` set).
3. Issues a brand-new access token + refresh token pair.

Because the old row is revoked before the new one is handed out, a
stolen-then-replayed refresh token simply fails to match on its next use —
this is rotation's core security property. What's deliberately **not**
implemented: reuse-chain tracking/cascading revocation (i.e. treating a
replayed old token as a signal to kill *all* of that user's other sessions).
That's a legitimate hardening step for later, but it adds real complexity
(distinguishing "expired" from "already used," walking/revoking a token
family) for a project at this stage. Skipping it means a detected replay is
simply rejected, not treated as an incident.

## Logout is per-device

`POST /auth/logout` revokes only the refresh token in the request's own
cookie. Other devices/sessions for that user are unaffected — this replaces
the old global `token_version` bump, which logged out every session at once.
There is no "log out everywhere" endpoint in this pass.

Logout does **not** require a valid access token. The refresh-token cookie is
itself the session identifier; requiring a still-valid (and now short-lived)
access token just to log out would be poor UX if it had already expired.
A missing/already-invalid cookie is treated as a no-op (200), not an error —
the desired end state ("this session is logged out") already holds.

One consequence worth calling out: logging out does **not** invalidate an
access token that was already issued and hasn't expired yet. It keeps working
until its own (≤15 minute) expiry. This is the accepted tradeoff of a fully
stateless access token — see `test_logout_revokes_refresh_token_only` in
`tests/auth_tests.rs`, which asserts this explicitly rather than leaving it
implicit.

## Storage

New `refresh_tokens` collection (`auth::models::RefreshTokenDoc`):
`_id`, `user_id`, `token_hash`, `created_at`, `expires_at`, `revoked_at`.

- `token_hash` has a unique index.
- `expires_at` has a TTL index (`expireAfterSeconds: 0`), so MongoDB's
  background reaper drops expired/spent rows on its own — no application-level
  cleanup job needed.

See `db::ensure_indexes` for both.

## Endpoints

- `POST /auth/register`, `POST /auth/login` — unchanged request/response
  shape (`{user, jwt}` in the body), now additionally set the refresh cookie.
- `GET /auth/me` — unchanged, still requires a valid access token.
- `POST /auth/refresh` — **new**. Reads the refresh cookie, returns a new
  access token (`{jwt}`) and rotates the cookie. 401 if the cookie is
  missing, unknown, expired, or already used.
- `POST /auth/logout` — revokes the current session's refresh token and
  clears the cookie.

## Known limitations / follow-ups

- No "log out of all devices" action yet (would mean revoking every
  `refresh_tokens` row for a `user_id` instead of one row by hash).
- No reuse-cascade detection on refresh token replay (see Rotation policy
  above).
