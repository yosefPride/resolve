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
- System Admin cannot appoint a successor on a group's behalf. Deleting a user who is the sole Group Admin of any group is blocked (see docs/api.md, `DELETE /admin/users/:id`) until an actual member of that group resolves it via normal group-scoped endpoints.

---

# Enforcement Mechanism

RBAC is enforced via:

- Actix middleware (request-level checks)
- Service-layer validation (business logic safety checks)

---

# Design Principle

Permissions are additive, never subtractive.

System Admin is NOT omniscient across tenants and cannot access data outside their group membership.
