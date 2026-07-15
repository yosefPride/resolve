# RESOLVE Database Design

---

# Database Type

MongoDB (NoSQL)

- Flexible schema
- Fast iteration for beginners
- Document-based multi-tenancy support

---

# Core Design Principle

All data is scoped by:

- user_id
- group_id

group_id is mandatory for ALL business data.

This is the foundation of multi-tenancy isolation.

---

# Collections Overview

## users

Stores system users.

Fields:

- \_id
- email
- password_hash (bcrypt)
- name
- global_role (System Admin only)
- created_at

---

## refresh_tokens

One document per outstanding refresh-token session. Backs POST /auth/refresh and POST /auth/logout — see docs/api.md.

Fields:

- \_id
- user_id
- token_hash (SHA-256 of the raw token; the raw token itself is never stored, same principle as password_hash)
- created_at
- expires_at
- revoked_at (nullable; set on rotation or logout)

A document is single-use: once revoked (by refresh or logout), it is never matched again. Expired/revoked documents are not queried against user-facing business logic and are not group-scoped — this collection is session data tied to a user, not tenant data.

---

## groups

Represents tenant isolation boundary.

Fields:

- \_id
- name
- owner_id (user who created the group; informational only — not used for authorization)
- created_at

Creating a group inserts a group_members row for the creator with role = Group Admin in the same operation. All authorization checks use group_members.role, never owner_id.

---

## group_members

Defines RBAC inside a group.

Fields:

- \_id
- group_id
- user_id
- role (Contributor | Group Admin)
- joined_at

A group must always have at least one member with role = Group Admin (except when the group itself is deleted).

---

## tickets

Core entity of the system.

Fields:

- \_id
- group_id
- ticket_number (running number scoped to group_id — the first ticket in a
  group is 1, independent of other groups' numbering; sourced from `counters`)
- title
- description
- status (open | closed)
- priority (low | high | critical)
- created_by
- created_at
- updated_at

No assignee field: tickets are not assigned to a user.

Only a Group Admin may edit a ticket after creation (including status changes)
— not even the creator, once a Contributor, may edit their own ticket. See
docs/rbac.md and docs/api.md (`PATCH /groups/{id}/tickets/{ticket_id}`).

---

## counters

Backs the per-group `ticket_number` sequence. One document per group.

Fields:

- \_id (== group_id)
- ticket_seq (last-assigned ticket_number for this group)

Incremented atomically via `find_one_and_update` + `$inc` on ticket creation —
avoids a race between two tickets created in the same group at once. Deleted
along with the group's tickets when the group is deleted.

---

## comments

Ticket discussions.

Fields:

- \_id
- group_id
- ticket_id
- user_id
- content
- created_at

---

## ai_ticket_insights

Stores AI-generated results per ticket.

Fields:

- \_id
- group_id
- ticket_id
- summary
- severity_prediction
- suggested_fix
- classification
- created_at
- updated_at

---

## ai_group_reports

Stores aggregated AI reports (Group Admin only).

Fields:

- \_id
- group_id
- report_data
- generated_at
- generated_by (user_id)

---

## admin_audit_log

Records System Admin's scoped exception to group self-governance: naming a Group Admin successor (or auto-deleting a group with no possible successor) as part of deleting a user — see docs/rbac.md ("Group Admin Succession") and docs/api.md (`POST /admin/users/:id/delete`).

Fields:

- \_id
- action (succession | group_auto_deleted)
- group_id
- deleted_user_id (the user being deleted, was sole Group Admin of group_id)
- successor_user_id (nullable; set only when action = succession)
- performed_by (System Admin's user_id)
- created_at

Like refresh_tokens, this is system-level data tied to an admin action, not group-scoped tenant data — it is written by System Admin, not queried by group-scoped business logic.

---

# Relationship Model (Important)

- users → refresh_tokens (1-to-many)
- users ↔ groups → many-to-many via group_members
- groups → tickets (1-to-many)
- tickets → comments (1-to-many)
- tickets → ai_ticket_insights (1-to-1 or 1-to-many over time)
- groups → ai_group_reports (1-to-many)
- users → admin_audit_log (deleted_user_id, performed_by) (1-to-many)
- groups → admin_audit_log (1-to-many)

---

# Multi-Tenancy Rule (CRITICAL)

Every query MUST include:

- group_id filter

Example rule:

NEVER:

- query tickets without group_id

ALWAYS:

- query tickets WHERE group_id = current_group

This ensures strict data isolation between companies.

---

# RBAC Storage Model

RBAC is stored in:

group_members.role

Roles:

- Contributor
- Group Admin

Rules:

- Role is per-group (not global)
- A user can have different roles in different groups (e.g. Group Admin in one group, Contributor in another)
- A group always has at least one Group Admin (see "Group Admin Succession" in docs/rbac.md)

---

# System Admin Model

System Admin is a GLOBAL role stored in users.global_role

System Admin capabilities:

- manage users
- manage groups
- view system metadata

System Admin limitations:

- cannot access group tickets unless member
- cannot bypass group isolation
- cannot appoint a Group Admin successor on a group's behalf (see docs/rbac.md, "Group Admin Succession")

---

# AI Data Strategy

AI results are stored per group:

- never global
- never cross-group
- always tied to ticket or group

AI data is:

- cached (avoid repeated API costs)
- optional (system still works without AI)

---

# Indexing Strategy (Important for performance)

Recommended indexes:

## users

- email (unique)

## refresh_tokens

- token_hash (unique)
- expires_at (TTL index, expireAfterSeconds: 0 — expired/spent documents are dropped automatically, no cleanup job needed)

## groups

No secondary indexes: nothing queries groups by anything but _id (owner_id is
informational only and never filtered on).

## group_members

- group_id + user_id (compound, unique — one membership row per user per group;
  also serves every per-group membership/role check)
- user_id (serves the "list my groups" lookups, which the compound index above
  cannot — user_id isn't its prefix)

## tickets

- group_id (critical)
- group_id + status
- group_id + created_by
- group_id + ticket_number (compound, unique)

## comments

- ticket_id
- group_id

## ai_ticket_insights

- ticket_id
- group_id

## admin_audit_log

- group_id (deferred — add when an audit-log read endpoint ships; nothing
  queries this collection yet and it only grows on rare succession events)
- deleted_user_id (deferred, same reason)

---

# Security Rules (Database Level)

Even though backend enforces it, DB design supports:

- group_id mandatory field enforcement
- no cross-group references without validation
- avoid global queries in services

---

# Scaling Strategy (Simple)

Start:

- single MongoDB instance

Later (if needed):

- sharding by group_id
- caching layer for tickets
- AI result caching layer

---

# Design Philosophy

- Keep schema simple
- Optimize for clarity, not perfection
- Avoid premature normalization
- Enforce isolation through group_id everywhere
