# Backend — System Admin

Covers `src/admin/`: `models.rs` (121), `repository.rs` (80), `service.rs` (308),
`handlers.rs` (92).

This is the hardest module. It implements the **one narrow exception** to group
self-governance: System Admin may resolve Group Admin succession, but only as a side effect
of deleting a user. Read `04-groups.md` first — this module manipulates group membership
directly, bypassing `GroupService`.

---

## The scope rule this module implements

System Admin is a **global** role (`users.global_role`), completely independent of group
roles. It can see:

- the user list (`GET /admin/users`)
- the group list, **metadata only** (`GET /admin/groups`)
- the succession audit trail (`GET /admin/audit-log`)

It **cannot** read tickets, comments, or a group's membership roster — there is no admin
endpoint for any of those. The single exception is the `eligible_successors` list surfaced
during a sole-Group-Admin deletion, which is the minimum information needed to make the
required choice.

Note `AdminService` holds a `GroupRepository` directly rather than a `GroupService`. That's
deliberate: the succession flow must do things `GroupService` forbids (reassign a role it
doesn't own, delete a group without being a member), so it cannot route through the group
business rules.

---

## `admin/models.rs`

### `enum AuditAction { Succession, GroupAutoDeleted }`
`snake_case` → `"succession"` / `"group_auto_deleted"`. The frontend's `AuditLogPanel`
maps these two strings to display labels.

### `struct AuditLogEntry`
The stored document in `admin_audit_log`:

`id`, `action`, `group_id`, `group_name`, `deleted_user_id`, `deleted_user_name`,
`successor_user_id: Option`, `successor_user_name: Option`, `performed_by`,
`performed_by_name`, `created_at`.

The `*_name` fields are **denormalized snapshots captured at write time, not lookups**. The
reasoning is in the comment and is the key insight of this collection: by the time anyone
reads the log, the deleted user (always) and an auto-deleted group (when
`action = group_auto_deleted`) no longer exist, so their ids cannot be resolved to names
after the fact.

Every `*_name` field carries `#[serde(default)]`, so entries written before those fields
existed still deserialize — they read back with an empty name rather than failing the whole
query. That's a schema-migration accommodation living in the type.

### `struct AuditLogEntryResponse` + `impl From<AuditLogEntry>`
Client shape: ObjectIds → hex strings, `created_at` → `DateTime<Utc>`.

### Query DTOs
- `AuditLogQuery { group_id: Option<String>, user_id: Option<String> }` — both optional and independent. `user_id` filters on the **deleted** user, not the performing admin.
- `AdminListQuery { search: Option<String> }` — shared by `GET /admin/users` and `GET /admin/groups`. The comment notes it will grow `page`/`per_page` when pagination lands.

### Deletion-flow DTOs
- `BlockedGroupInfo { group_id, group_name, eligible_successors: Vec<MemberResponse> }` — reuses the group module's `MemberResponse`.
- `AutoDeleteGroupInfo { group_id, group_name }`
- `DeletionCheckResponse { blocked_groups, auto_delete_groups }`
- `DeleteUserRequest { successors: HashMap<String, String> }` — group_id (hex) → successor user_id (hex). Only groups in `blocked_groups` need an entry.

---

## `admin/repository.rs`

### `enum AdminRepoError { Database(_) }`
Single variant → always `ApiError::Internal`.

### `struct AdminRepository { audit_log: Collection<AuditLogEntry> }`
**Only the audit log.** Everything else this module touches goes through `GroupRepository`
and `UserService`. Worth calling out — the admin module has no collections of its own beyond
this one.

### `async fn insert_audit_entry(&self, entry) -> Result<AuditLogEntry, AdminRepoError>`
Inserts and returns the entry with its generated `_id`.

### `async fn list_audit_log(&self, group_id: Option<ObjectId>, deleted_user_id: Option<ObjectId>) -> Result<Vec<AuditLogEntry>, _>`
Builds a filter from whichever options are present (both absent → whole log), sorted
`{created_at: -1}` (newest first). Each filter field has its own single-field index.

---

## `admin/service.rs`

### `struct DeletionPlan` (private, `#[derive(Default)]`)
The classification result — three buckets, and understanding them *is* understanding this module:

| Field | Meaning |
|---|---|
| `blocked: Vec<(ObjectId, String, Vec<GroupMember>)>` | Target is the **sole** Group Admin **and** other members exist. Carries `(group_id, group_name, other_members)`. Requires an explicitly named successor. |
| `auto_delete: Vec<(ObjectId, String)>` | Target is the sole Group Admin **and** the only member. No successor is possible, so the group is deleted outright. |
| `plain_removals: Vec<ObjectId>` | Target is a Contributor, **or** a Group Admin alongside other admins. Nothing to preserve — just drop the membership. |

### `struct AdminService { group_repo, user_service, admin_repo, rbac }`

### `async fn build_plan(&self, target_user_id) -> Result<DeletionPlan, ApiError>` (private)
The classifier, shared by both the preview and the commit.

For each group in `group_repo.list_groups_for_user(target_user_id)`:
1. `find_member(group_id, target_user_id)` → `ok_or(Internal)` (a listed group must have a membership; otherwise data is inconsistent).
2. `role != GroupAdmin` → `plain_removals`, continue.
3. `count_group_admins(group_id) > 1` → `plain_removals`, continue.
4. `list_members`, filter out the target → `others`.
5. `others.is_empty()` → `auto_delete`, else → `blocked`.

### `async fn deletion_check(&self, caller_id, target_user_id) -> Result<DeletionCheckResponse, ApiError>`
`require_system_admin` → confirm the target exists (`ok_or(NotFound)`) → `build_plan` →
enrich blocked groups' `others` into `MemberResponse` via `enrich_members`.

Read-only. Purely a preview.

### `async fn delete_user(&self, caller_id, target_user_id, successors: HashMap<ObjectId, ObjectId>) -> Result<(), ApiError>`

The most careful function in the codebase. Four phases, in this order:

**Phase 1 — authorize and snapshot.**
`require_system_admin` → load target (`ok_or(NotFound)`) → capture `deleted_user_name` and
`performed_by_name` **now, while the entities still exist**, for the audit entries written later.

**Phase 2 — re-derive and validate, before any write.**
Calls `build_plan` again rather than trusting the client's copy of the check — group
membership may have shifted since. Then, for every blocked group:
- a successor must be present in the `successors` map, else `Conflict("a successor is required for group …")`
- that successor must still appear in that group's `others`, else `Conflict("successor is not a member of group …")`

Both are `409`. **Nothing has been written at this point**, so a rejection leaves the system
completely untouched — no partial deletion.

**Phase 3 — execute, sequentially.**
- Per blocked group: look up the successor's name → `update_member_role(group, successor, GroupAdmin)` → `delete_member(group, target)` → `insert_audit_entry(action: Succession, successor_user_id: Some, successor_user_name: Some)`.
- Per auto-delete group: `delete_members_by_group` → `delete_group` → `insert_audit_entry(action: GroupAutoDeleted, successor fields: None)`.
- Per plain removal: `delete_member`.

**Phase 4 — `user_service.delete(target_user_id)` last.**

The ordering is the whole point, and it's stated in the comment: this is **not a Mongo
transaction** (the same choice as `GroupService::create_group`). Because the user document
is deleted last, a mid-failure leaves the target still existing with some memberships
already resolved — so simply re-running the deletion is safe and converges. Deleting the
user first would strand the remaining groups with no way to re-derive the plan.

Note: the audit entry for a blocked group is written **after** the role change and removal,
so a crash between them loses the log line for a change that did happen. And there is **no
guard against an admin deleting their own account** — see [`../deviations.md`](../deviations.md).

### `async fn list_users(&self, caller_id, search) -> Result<Vec<UserResponse>, ApiError>`
`require_system_admin` → `user_service.list_all(search)`. Case-insensitive substring on name
**or** email.

### `async fn list_groups(&self, caller_id, search) -> Result<Vec<GroupResponse>, ApiError>`
`require_system_admin` → `group_repo.list_all_groups(search)` → `GroupResponse`.
Metadata only — no membership, no counts, consistent with group isolation.

### `async fn list_audit_log(&self, caller_id, group_id, deleted_user_id) -> Result<Vec<AuditLogEntryResponse>, ApiError>`
`require_system_admin` → `admin_repo.list_audit_log` → map to response. Newest-first.

### `async fn delete_group(&self, caller_id, group_id) -> Result<(), ApiError>`
`require_system_admin` → `delete_members_by_group` → `delete_group` → `if !deleted { NotFound }`.

The comment explains why there's **no membership or succession check** here, unlike
`delete_user`: deleting the whole group removes the "at least one Group Admin" invariant
along with it, so there's no continuity to preserve. Group Admins deleting their own group
use `GroupService::delete_group` instead; this is the System-Admin-as-non-member path.

Also note: this is **not audit-logged** (only succession and auto-deletion are), and like the
group-module counterpart it leaves the group's tickets and `counters` row behind.

### `async fn enrich_members(&self, members: Vec<GroupMember>) -> Result<Vec<MemberResponse>, ApiError>` (private)
Same name/email join as `GroupService::enrich_member`, **duplicated rather than shared** —
the comment justifies it: `AdminService` already holds its own `UserService`, and this is
the only place it needs the join.

---

## `admin/handlers.rs`

All five handlers take `SystemAdminUser`, so the global-role check happens before any body runs.

### `fn parse_id(raw) -> Result<ObjectId, ApiError>`
Same helper as the other modules.

| Handler | Extractors | Flow |
|---|---|---|
| `deletion_check` | `Path<String>` | `parse_id` → `deletion_check` → `200 DeletionCheckResponse` |
| `delete_user` | `Path<String>`, `Json<DeleteUserRequest>` | `parse_id(target)`; then **parses every key and value of the `successors` map** into `ObjectId` (a malformed id → `400`) → `delete_user` → `204` |
| `list_users` | `Query<AdminListQuery>` | trims `search`, drops it if empty → `list_users` → `200` |
| `list_groups` | `Query<AdminListQuery>` | same trim/drop → `list_groups` → `200` |
| `list_audit_log` | `Query<AuditLogQuery>` | `parse_id` on each present filter via `.map(parse_id).transpose()?` → `list_audit_log` → `200` |
| `delete_group` | `Path<String>` | `parse_id` → `delete_group` → `204` |

The `.as_deref().map(str::trim).filter(|s| !s.is_empty())` idiom in the two list handlers is
what makes `?search=` (blank) behave identically to omitting the parameter.

---

## Test coverage

The admin module is the most heavily tested part of the backend — three tiers:

- `tests/admin_api_tests.rs` (1073 lines) — end-to-end HTTP
- `tests/admin_service_tests.rs` (702 lines) — service logic, especially plan classification
- `tests/admin_repository_tests.rs` (206 lines) — audit-log persistence and filtering

Reasonable, given this is the one place where a bug crosses tenant boundaries or destroys data.
