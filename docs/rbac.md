# RBAC System

---

# Roles

- Contributor
- Manager
- Administrator

---

# Scope

RBAC is GROUP-SCOPED.

Each user has a role per group, defined in:

group_members.role

A user may have different roles in different groups.

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

# Enforcement Mechanism

RBAC is enforced via:

- Actix middleware (request-level checks)
- Service-layer validation (business logic safety checks)

---

# Design Principle

Permissions are additive, never subtractive.

Admin is NOT omniscient across tenants and cannot access data outside their group membership.
