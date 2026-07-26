# Backend — RBAC & Request Middleware

Covers: `src/server/middleware.rs` (198 lines) and `src/rbac/service.rs` (88 lines).

These two files are where authentication turns into authorization. Together they're under
300 lines and every feature module depends on them, so this is the highest-leverage part of
the backend to understand.

---

## The two-layer model

Authorization runs **twice** on every protected request, deliberately:

| Layer | Where | Runs when |
|---|---|---|
| Request level | `server/middleware.rs` extractors | Before the handler body executes at all |
| Service level | `RbacService` calls inside each `*Service` method | Inside the business logic |

The service layer is not redundant defensiveness for its own sake — it means a handler is
never the only thing standing between a request and the data. If someone later mounts a
service method on a new route and forgets the extractor, the check still runs.

**Group isolation is a third, separate mechanism** and is *not* part of RBAC: it's the
repository rule that every tenant-data query filters on `group_id`. RBAC stops the wrong
*person*; the `group_id` filter stops the wrong *id*.

---

## `src/server/middleware.rs`

Despite the filename, there is **no Actix middleware here** (no `Transform`/`Service` impls).
It contains three `FromRequest` **extractors** — types you put in a handler's signature, and
Actix resolves them before calling the handler. If resolution fails, the handler never runs
and the error is rendered directly.

### Shared helpers (private)

#### `fn authorization_header(req: &HttpRequest) -> Option<String>`
Pulls the `Authorization` header, converts to `&str` (fails on non-ASCII), owns it as a `String`.

#### `fn bearer_token(header: Option<String>) -> Result<String, ApiError>`
`None` → `Unauthenticated`. Present but not prefixed `"Bearer "` → `Unauthenticated`.
Otherwise returns the token.

The comment explains why this is kept separate from and **ahead of** any `AppState` access:
the "no token" / "wrong scheme" paths resolve without needing state, which is also what lets
the extractors be unit-tested with no live database.

#### `fn user_id_from_token(token: &str, secret: &str) -> Result<ObjectId, ApiError>`
`jwt::decode_token` (signature + exp), then `ObjectId::parse_str(&claims.sub)`. Both failure
modes collapse to `Unauthenticated`.

### The common `from_request` shape

All three extractors follow the same pattern, and the reason is worth knowing:

```rust
fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
    // 1. Pull everything needed out of `req` up front, as OWNED values
    let header = authorization_header(req);
    let state  = req.app_data::<web::Data<AppState>>().cloned();
    // 2. Return a boxed future that no longer borrows `req`
    Box::pin(async move { ... })
}
```

The returned `LocalBoxFuture` outlives the borrow of `req`, so nothing borrowed can be
captured. `type Error = ApiError` is what makes a failed extraction render as the standard
JSON error body.

---

### `struct AuthenticatedUser { user_id: ObjectId }`

**Fully stateless — no database lookup at all.** Verified by signature + `exp` only.

The doc comment states exactly why that's safe: access tokens are short-lived (15 min), so
revocation is the refresh token's job, and a stolen access token expires on its own. The
tradeoff is explicit — a user deleted mid-token keeps a working token until it expires,
which is why `me` re-checks existence and maps a missing user to `Unauthenticated`.

Resolution: `bearer_token` → `state` (else `Internal`) → `user_id_from_token`.

Used on: all `/auth` protected routes, `POST /groups`, `GET /groups`. That is, routes where
*authentication alone* is the requirement.

---

### `struct GroupScoped { user_id: ObjectId, group_id: ObjectId, role: Role }`

The tenant-scoped request context, and the single most important type in the backend.

Resolution:
1. `bearer_token` → `state` → `user_id_from_token` (same as above).
2. Reads the group id from `req.match_info().get("id")` — **by convention the path segment is always named `{id}`**.
   - Segment absent → `ApiError::Internal`. That means the extractor was mounted on a route without an `{id}`, i.e. a server misconfiguration — distinguished deliberately from a client-supplied bad id.
   - Segment present but unparseable → `ApiError::Validation("invalid id")`.
3. `RbacService::new(&state.db).require_member(group_id, user_id)` — **one DB lookup**, which both authorizes and yields the caller's current role.
4. Returns `{user_id, group_id, role}`.

Two consequences worth stating when explaining this:

- **Role is resolved per request, never carried in the JWT.** A member who is removed or demoted is rejected on their very next request, not at token expiry.
- **Handlers scoped to a group never parse a group id themselves.** They take `scoped.group_id` and hand it to services/repositories. There is one uniform place group scope enters the system.

Where two path segments exist (`/groups/{id}/users/{user_id}`, `/groups/{id}/tickets/{ticket_id}`),
the handler additionally takes `web::Path<(String, String)>` and **discards the first
element** in favor of `scoped.group_id` — you'll see `let (_, ticket_id) = path.into_inner();`
repeatedly. Same value, but taking it from the extractor keeps a single source of truth.

Used on: every `/groups/{id}/...` route.

---

### `struct SystemAdminUser { user_id: ObjectId }`

Global-scope guard for `/admin`. Resolution: authenticate, then
`RbacService::require_system_admin(user_id)` — a DB read of `users.global_role`.

A non-admin **or an unknown user** resolves to `Forbidden`, so a stale or forged token can't
be used to probe whether a user id exists.

Used on: all six `/admin` routes.

---

### Inline tests

Four `#[actix_web::test]` cases, all at the header layer so **none of them touch the
database**:
- `missing_authorization_header_is_rejected` (`AuthenticatedUser`)
- `non_bearer_authorization_header_is_rejected` (`AuthenticatedUser`, sends `Basic abc123`)
- `group_scoped_missing_header_is_rejected`
- `system_admin_missing_header_is_rejected`

The comment notes this is intentional coverage design: all three extractors share the same
header-first handling, so the no-token/wrong-scheme cases are covered once per extractor at
the cheapest layer.

---

## `src/rbac/service.rs`

### `struct RbacService { group_repo: GroupRepository, user_service: UserService }`

The doc comment defines its scope precisely — it answers exactly one kind of question
("what is this user's relationship to this group, or to the system?") and carries **no
feature-specific logic**, which is why tickets, groups, and admin can all share it instead
of growing private copies.

It also states what deliberately does *not* live here:
- **Group isolation** — enforced by repository-level `group_id` filters.
- **The sole-Group-Admin succession guard** — that's group-membership business logic, and stays in `GroupService::guard_sole_admin_removal`.

### `async fn require_member(&self, group_id, user_id) -> Result<GroupMember, ApiError>`
`group_repo.find_member(group_id, user_id)` → `ok_or(ApiError::Forbidden)`.

Two design points, both called out in the code:
- Not a member → **`Forbidden`, not `NotFound`** — this avoids telling a non-member whether the group id even exists. (`ApiError::Forbidden`'s message is correspondingly generic.)
- Returns the whole `GroupMember`, not `()`, so a caller needing a finer decision can check membership once and branch on `member.role` rather than querying twice.

### `async fn require_group_admin(&self, group_id, user_id) -> Result<GroupMember, ApiError>`
Calls `require_member`, then `if member.role != Role::GroupAdmin { return Forbidden }`.

### `async fn require_system_admin(&self, user_id) -> Result<(), ApiError>`
`user_service.find_by_id(user_id)` → `ok_or(Forbidden)` → matches
`Some(GlobalRole::SystemAdmin)` → `Ok(())`, everything else → `Forbidden`.
A missing user maps to the same `Forbidden` as a non-admin, for the probe-resistance reason above.

### `fn require_owner_or_group_admin(member: &GroupMember, resource_owner_id: ObjectId) -> Result<(), ApiError>`
**Associated function, not a method — pure, no `&self`, no DB access.** Takes an
already-resolved membership (so the caller does one `require_member` and reuses it) plus the
resource's creator id. Passes if `member.role == GroupAdmin || member.user_id == resource_owner_id`.

Important: **nothing currently calls this.** The comment describes it as the ticket/comment
rule, but `TicketService` uses `require_group_admin` for update/delete (Group Admin only,
regardless of creator), and the comment module is empty. It is written and ready for
comments, and unused today. See [`../deviations.md`](../deviations.md).

---

## Route → guard reference

| Route | Extractor | Additional service-level check |
|---|---|---|
| `POST /auth/register`, `/login`, `/refresh`, `/logout` | none | — |
| `GET/PATCH /auth/me`, `POST /auth/me/password` | `AuthenticatedUser` | — |
| `POST /groups` | `AuthenticatedUser` | — (any authenticated user may create) |
| `GET /groups` | `AuthenticatedUser` | — (returns only the caller's own memberships) |
| `GET /groups/{id}` | `GroupScoped` | `require_member` |
| `PATCH /groups/{id}` | `GroupScoped` | `require_group_admin` |
| `DELETE /groups/{id}` | `GroupScoped` | `require_group_admin` |
| `GET /groups/{id}/users` | `GroupScoped` | `require_member` |
| `GET /groups/{id}/users/lookup` | `GroupScoped` | `require_group_admin` |
| `POST /groups/{id}/users` | `GroupScoped` | `require_group_admin` |
| `PATCH /groups/{id}/users/{user_id}` | `GroupScoped` | `require_group_admin` (+ sole-admin guard on demotion) |
| `DELETE /groups/{id}/users/{user_id}` | `GroupScoped` | `require_group_admin` if removing someone else; **none** if removing yourself (+ sole-admin guard either way) |
| `POST /groups/{id}/tickets` | `GroupScoped` | `require_member` |
| `GET /groups/{id}/tickets` | `GroupScoped` | `require_member` |
| `GET /groups/{id}/tickets/{ticket_id}` | `GroupScoped` | `require_member` |
| `PATCH /groups/{id}/tickets/{ticket_id}` | `GroupScoped` | `require_group_admin` |
| `DELETE /groups/{id}/tickets/{ticket_id}` | `GroupScoped` | `require_group_admin` |
| all `/admin/*` | `SystemAdminUser` | `require_system_admin` |

The one asymmetric row is `DELETE /groups/{id}/users/{user_id}`: a single endpoint serves
both "admin removes a member" and "member leaves". The handler branches on
`target_user_id == scoped.user_id` and calls `leave_group` (no admin check) or
`remove_member` (admin check). Both paths run the sole-Group-Admin guard.
