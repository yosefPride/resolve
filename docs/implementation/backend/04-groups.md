# Backend — Groups & Membership

Covers `src/group/`: `models.rs` (107), `repository.rs` (218), `service.rs` (277),
`handlers.rs` (165).

The group is the tenant boundary. This module owns group metadata, membership, and
group-scoped roles — and it's the first module with genuine business rules (the
sole-Group-Admin invariant).

---

## `group/models.rs`

### `enum Role { Contributor, GroupAdmin }`
`#[serde(rename_all = "snake_case")]` → serializes as `"contributor"` / `"group_admin"`.
Derives `Copy, PartialEq, Eq`, so role comparisons are trivial and roles pass by value.

Contrast with `GlobalRole`, which has **no** rename and serializes as `"SystemAdmin"`. The
frontend's `utils/roles.js` encodes both conventions, which is the kind of asymmetry that
bites when you're explaining it.

### `struct Group`
The stored document: `id` (`_id`), `name`, `owner_id: ObjectId`, `created_at`.

`owner_id` records who created the group and is **never used for authorization** — every
permission decision reads `group_members.role`. It's informational only.

### `struct GroupMember`
The stored membership row: `id`, `group_id`, `user_id`, `role`, `joined_at`.
This is the RBAC table. One row per (group, user), enforced by a unique compound index.

### `struct CreateGroupInput { name, owner_id }`
Input DTO for the repository.

### Response shapes — three of them, deliberately distinct

#### `struct GroupResponse { id, name, owner_id, created_at }`
Plain group metadata. Returned by create / get / rename, and by the admin group list.
`impl From<Group> for GroupResponse` does the hex/RFC3339 conversion.

#### `struct GroupSummaryResponse { id, name, role, member_count, open_ticket_count, created_at }`
**Specifically for `GET /groups` ("my groups")**, where `role` (caller-relative) and the two
counts are meaningful. The code comment explains why this isn't merged into `GroupResponse`:
those fields make no sense for create/get/rename or for the admin's system-wide list.

Note it has **no `owner_id`** — the two shapes are not subsets of each other.

#### `struct MemberResponse { id, user_id, name, email, role, joined_at }`
A membership row **joined with the user's name/email**, which `GroupMember` doesn't carry.
Also reused by the admin module for eligible-successor lists.

### Request DTOs
- `CreateGroupRequest { name }` — used for **both** create and rename (the rename handler reuses it rather than defining a `RenameGroupRequest`).
- `AddMemberRequest { user_id: String, role: Role }` — `user_id` arrives as hex text and is parsed in the handler.
- `UpdateMemberRoleRequest { role: Role }`
- `LookupUserQuery { email: String }` — deserialized from the query string.
- `UserLookupResponse { id, name, email }` — deliberately minimal; it's just enough to confirm you found the right person before adding them.

---

## `group/repository.rs`

### `enum GroupRepoError { DuplicateMember, Database(_) }`
With `From<mongodb::error::Error>` routing through `is_duplicate_key`.

### `fn is_duplicate_key(err) -> bool`
Matches `ErrorKind::Write(WriteFailure::WriteError)` with code `11000`. Unlike the user
repository's version, this **only checks the write shape**, not the command shape — fine
here, because the only unique-index write is `insert_member`.

This is what makes duplicate-membership rejection atomic: rather than "check if member,
then insert" (racy), it just inserts and lets the unique index on
`(group_id, user_id)` fail, mapping 11000 → `DuplicateMember` → `409 conflict`.

### `struct GroupRepository { groups: Collection<Group>, members: Collection<GroupMember> }`
One repository, two collections. That's why `RbacService` and `AdminService` can hold a
single `GroupRepository` and reach membership data.

| Method | What it does |
|---|---|
| `new(db)` | Handles for `"groups"` and `"group_members"`. |
| `create_group(input)` | Inserts with `created_at: now`, returns the struct with the generated `_id`. |
| `find_group_by_id(id)` | By `_id`. |
| `list_all_groups(search)` | Admin-only. Blank/absent search → empty filter (all groups); otherwise `{name: substring_regex(term)}`. No pagination. |
| `delete_group(id)` | `delete_one`, returns `deleted_count > 0`. **Deletes only the group document.** |
| `rename_group(id, name)` | `update_one` + `$set`. Returns `modified_count > 0` — note this is `false` when renaming to the *same* name, since nothing was modified. |
| `insert_member(group_id, user_id, role)` | Inserts with `joined_at: now`. Relies on the unique index for duplicate rejection. |
| `find_member(group_id, user_id)` | The lookup behind every RBAC check. Filters on both fields. |
| `list_members(group_id)` | All rows for a group. |
| `list_groups_for_user(user_id)` | **Two queries**: fetch memberships by `user_id`, collect their `group_id`s, then `{_id: {$in: ids}}` against `groups`. Early-returns an empty `Vec` if there are no memberships (avoiding an `$in: []` query). |
| `list_memberships_for_user(user_id)` | Same first query, but keeps the `GroupMember` (and its role) instead of resolving to `Group`. Used by `list_my_groups`, which needs the caller's role per group. |
| `count_members(group_id)` | `count_documents`. |
| `count_group_admins(group_id)` | `count_documents` with `{group_id, role: <GroupAdmin bson>}`. Uses `bson::to_bson(&Role::GroupAdmin)` rather than a hardcoded string, so the serde rename stays the single source of truth. |
| `update_member_role(group_id, user_id, role)` | `$set` on role, returns `modified_count > 0` (so setting the same role returns `false`). |
| `delete_member(group_id, user_id)` | `delete_one` on both fields. |
| `delete_members_by_group(group_id)` | `delete_many`, returns the count. Used when a group is deleted. |

Every membership query filters on `group_id`, `user_id`, or both — never a bare scan.

---

## `group/service.rs`

### `struct GroupService { repo, ticket_repo, user_service, rbac }`
Note the **`ticket_repo` field** — this is the only cross-feature dependency in the backend,
and it exists for exactly one reason: `list_my_groups` reports `open_ticket_count`.

### `async fn create_group(&self, user_id, name) -> Result<GroupResponse, ApiError>`
No RBAC check — any authenticated user may create a group.
1. `repo.create_group(CreateGroupInput { name, owner_id: user_id })`.
2. `repo.insert_member(group_id, user_id, Role::GroupAdmin)`.

The comment is explicit that these are **two sequential writes, not a transaction**, and
that a failure of the second leaves a group with no members. It's accepted as
low-probability and cheap to detect/repair manually rather than adding session plumbing.
(The same choice is made in admin user deletion.)

### `async fn list_my_groups(&self, user_id) -> Result<Vec<GroupSummaryResponse>, ApiError>`
`list_memberships_for_user`, then **per membership**: `find_group_by_id`, `count_members`,
`ticket_repo.count_open_by_group`.

That's `1 + 3N` queries for N groups — an N+1 pattern, sequential (not concurrent). Fine at
the expected number of groups per user, but it's the honest answer if you're asked about
performance. A missing group for a live membership maps to `ApiError::Internal` (it would
mean dangling data).

### `async fn get_group(&self, user_id, group_id)`
`require_member` → `find_group_by_id` → `ok_or(NotFound)`.

### `async fn rename_group(&self, user_id, group_id, name)`
`require_group_admin` → `repo.rename_group` (return value ignored) → re-fetch and return.
Because the boolean is ignored, renaming to the identical name still returns `200` with the
group rather than a spurious error.

### `async fn delete_group(&self, user_id, group_id) -> Result<(), ApiError>`
`require_group_admin` → `delete_members_by_group` → `delete_group`.

**This deletes memberships and the group document only.** It does not delete the group's
tickets or its `counters` row. See [`../deviations.md`](../deviations.md) — this is the most
significant behavioral gap in the backend.

### `async fn list_members(&self, user_id, group_id) -> Result<Vec<MemberResponse>, ApiError>`
`require_member` → `list_members` → `enrich_member` per row.

### `async fn enrich_member(&self, member: GroupMember) -> Result<MemberResponse, ApiError>` (private)
The join that fills in `name`/`email`. One `user_service.find_by_id` **per member** — the
comment explicitly justifies this over a `$lookup` aggregation: it matches the rest of the
repository layer (no aggregations anywhere in the codebase) and is fine at expected group
sizes. A deleted user yields `("", "")` via `unwrap_or_default()` rather than failing.

### `async fn lookup_user_by_email(&self, user_id, group_id, email) -> Result<UserLookupResponse, ApiError>`
`require_group_admin` → `user_service.find_by_email` → `ok_or(NotFound)`.

The comment explains the product reasoning: there is no user directory and no join flow, so
an exact email match is the only way to resolve the `user_id` that `add_member` needs.
Being Group-Admin-only limits it as an email-enumeration oracle.

### `async fn add_member(&self, user_id, group_id, target_user_id, role)`
`require_group_admin` → `insert_member` → `enrich_member`.
Duplicate membership surfaces from the unique index as `409`.

Worth noting: it does **not** verify that `target_user_id` refers to an existing user. In
practice the id comes from `lookup_user_by_email`, but a hand-crafted request could insert a
membership row pointing at a nonexistent user — which `enrich_member` then renders with
empty name/email.

### `async fn update_member_role(&self, user_id, group_id, target_user_id, role)`
1. `require_group_admin`.
2. **If demoting to `Contributor`** → `guard_sole_admin_removal` (demoting the last admin is blocked exactly like removing them).
   **If promoting to `GroupAdmin`** → just confirm the member exists (`find_member` → `ok_or(NotFound)`).
3. `update_member_role` → `if !updated { NotFound }`.
4. Re-fetch and `enrich_member`.

Step 3's reliance on `modified_count` means setting a member's **current** role returns
`404 NotFound` rather than a no-op success. A small rough edge in the semantics.

### `async fn remove_member(&self, user_id, group_id, target_user_id) -> Result<(), ApiError>`
`require_group_admin` → `guard_sole_admin_removal` → `delete_member` → `if !deleted { NotFound }`.

### `async fn leave_group(&self, user_id, group_id) -> Result<(), ApiError>`
**No admin check** — you may always remove yourself. Still runs
`guard_sole_admin_removal`, so a sole Group Admin cannot walk away and orphan the group.

### `async fn guard_sole_admin_removal(&self, group_id, target_user_id) -> Result<(), ApiError>` (private)
The invariant enforcer:
1. `find_member` → `ok_or(NotFound)`.
2. If the target's role is `GroupAdmin`, `count_group_admins(group_id)`; if `<= 1`, return `ApiError::Conflict("a successor Group Admin must be appointed before the sole Group Admin can be removed")` → `409`.

Called by `remove_member`, `leave_group`, and the demotion branch of `update_member_role` —
the three ways a group could lose its last admin. Deleting the whole group deliberately
bypasses it, since the invariant disappears with the group.

---

## `group/handlers.rs`

### Helpers
- `fn parse_id(raw: &str) -> Result<ObjectId, ApiError>` — `ObjectId::parse_str`, mapping failure to `Validation("invalid id")`.
- `fn validate_name(name: &str) -> Result<(), ApiError>` — rejects blank/whitespace-only. **No maximum length** (contrast with ticket titles, which are capped at 200).

### Handlers

| Handler | Extractor | Flow |
|---|---|---|
| `create_group` | `AuthenticatedUser` | `validate_name` → `create_group` → `201` |
| `list_my_groups` | `AuthenticatedUser` | `list_my_groups` → `200` (array of `GroupSummaryResponse`) |
| `get_group` | `GroupScoped` | `get_group` → `200` |
| `rename_group` | `GroupScoped` | `validate_name` → `rename_group` → `200` |
| `delete_group` | `GroupScoped` | `delete_group` → `204` |
| `list_members` | `GroupScoped` | `list_members` → `200` |
| `lookup_user` | `GroupScoped` + `Query<LookupUserQuery>` | rejects blank email with `400` → `lookup_user_by_email` → `200` |
| `add_member` | `GroupScoped` + `Json<AddMemberRequest>` | `parse_id(user_id)` → `add_member` → `201` |
| `update_member_role` | `GroupScoped` + `Path<(String,String)>` + `Json` | discards path element 0, parses `target_user_id` → `update_member_role` → `200` |
| `remove_member` | `GroupScoped` + `Path<(String,String)>` | **branches**: `target == scoped.user_id` → `leave_group`, else `remove_member` → `204` |

The block comment above `get_group` states the module's convention: the `{id}`-scoped
handlers take `GroupScoped` rather than `AuthenticatedUser`, the extractor has already
authenticated + parsed + verified membership, and the services still re-run their own role
checks underneath — request-level enforcement is an additional layer, not a replacement.

`remove_member`'s comment makes the dual-purpose design explicit: one endpoint covers both
admin-driven removal and self-service leaving, because removing yourself doesn't require
being a Group Admin while removing someone else does — and both paths still hit the
sole-Group-Admin guard.
