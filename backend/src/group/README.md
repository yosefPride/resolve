# Group Module

Implements groups — the tenant/isolation boundary for the system (see `docs/database.md`,
`docs/rbac.md`) — and their membership/roles. This is build-order step 2 (see root
`CLAUDE.md`), sitting directly on top of auth and below RBAC middleware, tickets, and
comments.

## Layering

Follows the same `Request → Handler → Service → Repository → Database` pattern as every
other feature module:

- **`models.rs`** — `Group`, `GroupMember`, `Role`, and the request/response DTOs. No behavior.
- **`repository.rs`** — `GroupRepository`. Raw MongoDB access only (`groups` and
  `group_members` collections). No authorization or business rules.
- **`service.rs`** — `GroupService`. All business logic and authorization checks.
- **`handlers.rs`** — thin Actix handlers. Parse the request, call the service, map the
  result to an `HttpResponse`. No logic beyond input parsing/validation.
- **`mod.rs`** — re-exports the four submodules.

## Data model

```rust
pub enum Role { Contributor, GroupAdmin }          // serialized as "contributor" / "group_admin"

pub struct Group {
    id: Option<ObjectId>,
    name: String,
    owner_id: ObjectId,   // informational only — never used for authorization, see docs/database.md
    created_at: BsonDateTime,
}

pub struct GroupMember {
    id: Option<ObjectId>,
    group_id: ObjectId,
    user_id: ObjectId,
    role: Role,
    joined_at: BsonDateTime,
}
```

`GroupResponse`/`MemberResponse` are the HTTP-facing shapes (hex-string ids, `chrono::DateTime<Utc>`
timestamps), mirroring how `user::models::UserResponse` wraps `User`.

## Authorization model

RBAC enforcement is **inline in `GroupService`**, not middleware — the dedicated RBAC
middleware module (`src/rbac/`) is a later build-order step. Every mutating/reading method
starts with one of two private helpers:

- `require_member(group_id, user_id)` — must have a `group_members` row, or `ApiError::Forbidden`
- `require_group_admin(group_id, user_id)` — must additionally have `role == GroupAdmin`, or `ApiError::Forbidden`

**Deliberate design choice:** a non-member gets `Forbidden` (403), never `NotFound` (404),
even when the group doesn't exist at all. This stops a non-member from telling the two cases
apart and enumerating group ids by probing status codes.

### Sole-Group-Admin guard

`guard_sole_admin_removal(group_id, target_user_id)` is shared by `remove_member`,
`update_member_role` (on demotion), and `leave_group`. It blocks the operation with
`ApiError::Conflict` whenever the target is the group's *only* `GroupAdmin`
(`GroupRepository::count_group_admins <= 1`) — a successor must be promoted first, or the
group deleted outright. See `docs/rbac.md`, "Group Admin Succession".

### Sequential writes, not transactions

`create_group` does two separate writes (insert the group, then insert the creator as
`GroupAdmin`) rather than a Mongo session/transaction — same tradeoff made for the
admin-triggered user-deletion flow (see `docs/rbac.md`). If the second write fails, the
group is left with no members; considered low-probability and cheap to notice/retry rather
than worth adding transaction plumbing for.

## Service methods → repository calls

| `GroupService` method | Auth check | Notes |
|---|---|---|
| `create_group` | none (any authenticated user) | creator becomes `GroupAdmin` |
| `list_my_groups` | none | scoped to caller via `group_members` |
| `get_group` | member | |
| `rename_group` | admin | |
| `delete_group` | admin | cascades `group_members` first |
| `list_members` | member | |
| `add_member` | admin | duplicate add → `Conflict` (unique index on `group_id`+`user_id`) |
| `update_member_role` | admin | demoting the sole admin → sole-admin guard |
| `remove_member` | admin | target may be anyone, including self; sole-admin guard |
| `leave_group` | **none** (self-service) | caller removes themselves; sole-admin guard still applies |

## HTTP endpoints (`handlers.rs` + `server/routes.rs`)

| Method | Path | Handler | Service call |
|---|---|---|---|
| POST | `/groups` | `create_group` | `create_group` |
| GET | `/groups` | `list_my_groups` | `list_my_groups` |
| GET | `/groups/{id}` | `get_group` | `get_group` |
| PATCH | `/groups/{id}` | `rename_group` | `rename_group` |
| DELETE | `/groups/{id}` | `delete_group` | `delete_group` |
| GET | `/groups/{id}/users` | `list_members` | `list_members` |
| POST | `/groups/{id}/users` | `add_member` | `add_member` |
| PATCH | `/groups/{id}/users/{user_id}` | `update_member_role` | `update_member_role` |
| DELETE | `/groups/{id}/users/{user_id}` | `remove_member` | `leave_group` if `user_id == caller`, else `remove_member` |

All routes require `AuthenticatedUser` (the stateless JWT extractor from `server/middleware.rs`).
Path segments are hex `ObjectId` strings; malformed ids fail with `ApiError::Validation` (400)
before the service layer is even called.

The last row is a single endpoint covering two different authorization rules: removing
*someone else* requires `GroupAdmin`; removing *yourself* requires nothing but membership.
This resolves an ambiguity in `docs/api.md`'s original endpoint description ("Group Admin
only") against having a separate self-service `leave_group` in the service layer.

## Known gaps / not yet implemented

- **System Admin group visibility** — `docs/api.md` says System Admin can `GET /groups/:id`
  for any group (metadata only). `get_group` doesn't implement this exception yet; a System
  Admin who isn't a member currently gets `Forbidden` like anyone else.
- **`POST /groups/:id/join`** — self-service join was scoped out early on (see project
  memory / conversation history); not implemented.
- **Active group JWT claim** — `docs/backend.md`'s "Active Group Concept" (active group
  stored in the JWT, auto-scoping business logic) isn't implemented. Every group method
  takes an explicit `group_id` argument instead.
- **RBAC middleware** — the inline checks in `GroupService` are a stand-in for the general
  RBAC middleware module (`src/rbac/`), which is a later build-order step.

## Tests

Real-MongoDB integration tests (no mocking), against the `resolve_test` Atlas database —
**must be run with `-- --test-threads=1`** (parallel runs race on shared collection drops;
see `tests/group_repository_tests.rs`'s `setup()`):

- `tests/group_repository_tests.rs` — repository layer, 15 tests
- `tests/group_service_tests.rs` — business logic + every authorization/guard path, 18 tests
- `tests/group_api_tests.rs` — full HTTP flow through real routes (`actix_web::test`), 6 tests
