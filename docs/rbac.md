# RBAC System

---

# Roles

This document covers GROUP-SCOPED roles only. The System Admin global role is documented separately in docs/database.md ("System Admin Model") and CLAUDE.md.

- Contributor
- Group Admin

---

# Scope

RBAC is GROUP-SCOPED.

Each user has a role per group, defined in:

group_members.role

A user may have different roles in different groups (e.g. Group Admin in one group, Contributor in another).

---

# Enforcement Rules

- All permissions enforced on backend only
- Frontend is only for UX hiding
- Every group-scoped request must validate:
  - authentication (JWT)
  - membership in the group named in the request path
  - role within that group

---

# Permission Model

Permissions are static and role-based.

No dynamic policy system exists.

---

# Group Admin Succession

- A group must always have at least one Group Admin.
- A Group Admin may add new members as either Contributor or Group Admin.
- A Group Admin may not leave, or be removed, while they are the sole Group Admin of a group — a successor Group Admin must be appointed first, unless the entire group is being deleted.
- System Admin cannot change group roles under normal circumstances. The one exception is scoped to user deletion: when deleting a user who is the sole Group Admin of a group, System Admin must explicitly name a successor from that group's existing members (see docs/api.md, `GET /admin/users/:id/deletion-check` and `POST /admin/users/:id/delete`); if the user is that group's only member, the group is deleted automatically instead. This action is audit-logged (see docs/database.md, `admin_audit_log`).
- Deleting the group itself is a separate, simpler case with no successor requirement at all: System Admin may delete any group outright via `DELETE /admin/groups/:id` (see docs/api.md), with no membership or successor check, since removing the whole group removes the "at least one Group Admin" invariant along with it. This is not audit-logged the way user-deletion succession is — it doesn't reassign any group-internal role, it just removes the tenant. Group Admins deleting their own group use the separate, pre-existing `DELETE /groups/:id` instead.

---

# Enforcement Mechanism

RBAC is enforced in two layers, both of which always run (defense in depth):

## Request level — Actix extractors (src/server/middleware.rs)

- `GroupScoped` — for group-scoped routes (`/groups/{id}/...`). Reads the group
  id from the path, verifies membership, and resolves the caller's current role
  in one lookup. Non-member → 403. There is no "active group": scope is always
  the path's group id, and role is resolved per request (never carried in the
  JWT), so a removed/demoted member is rejected on their next request.
- `SystemAdminUser` — for `/admin` routes. Confirms the caller holds the System
  Admin global role. Non-admin → 403.

## Service level — RbacService (src/rbac/service.rs)

Services re-run the check via shared helpers, so a handler is never the only
thing standing between a request and the data:

- `require_member(group_id, user_id)` — returns the `GroupMember` (with role)
- `require_group_admin(group_id, user_id)` — member AND Group Admin
- `require_system_admin(user_id)` — global System Admin role
- `require_owner_or_group_admin(member, resource_owner_id)` — the ticket/comment
  rule: a Contributor may act only on resources they created, a Group Admin on
  any resource in the group

## Not part of RBAC: group isolation

Keeping one group's data unreachable through another group's id is enforced
separately, by the repository rule that every group-scoped query filters by
`group_id` (docs/backend.md) — a mismatched resource id simply isn't found. It
is not one of the role helpers above.

---

# Design Principle

Permissions are additive, never subtractive.

System Admin is NOT omniscient across tenants and cannot access data outside their group membership.
