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
- Every request must validate:
  - authentication (JWT)
  - active group
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

RBAC is enforced via:

- Actix middleware (request-level checks)
- Service-layer validation (business logic safety checks)

---

# Design Principle

Permissions are additive, never subtractive.

System Admin is NOT omniscient across tenants and cannot access data outside their group membership.
