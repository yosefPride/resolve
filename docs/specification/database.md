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

## ticket_activity

Read-only history of ticket-level mutations — see docs/api.md, "Activity Endpoint".

Fields:

- \_id
- group_id
- ticket_id
- actor_id
- event_type (ticket_created | status_changed | priority_changed | title_changed | description_changed | comment_added | comment_deleted | link_added | link_removed)
- old_value, new_value (nullable — populated only for status/priority/title changes and link_added/link_removed)
- comment_id (nullable — populated only for comment_added/comment_deleted)
- link_kind (nullable — relation | reference; populated only for link_added/link_removed)
- occurred_at

Written exclusively as a side effect of ticket/comment/link/reference mutations elsewhere in the system, never accepted from a client. One entry per changed field, not one entry per request — an update touching both status and priority writes two documents.

---

## ticket_links

Relates two tickets in the same group — see docs/api.md, "Link Endpoints".

Fields:

- \_id
- group_id
- source_ticket_id
- target_ticket_id
- relation_type (blocks | relates_to | duplicates)
- created_by
- created_at

Stored directionally from source_ticket_id's viewpoint; the inverse label (e.g. is_blocked_by) is resolved at read time, never stored as a second document. relates_to is symmetric — enforced at the service layer so an A↔B pair is never represented by two documents; blocks and duplicates are directional, so both directions may coexist as separate documents.

---

## ticket_references

An external URL attached to a single ticket — see docs/api.md, "Reference Endpoints".

Fields:

- \_id
- group_id
- ticket_id
- label
- url
- created_by
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

## admin_audit_log

Records System Admin actions worth a trail: naming a Group Admin successor (or
auto-deleting a group with no possible successor) as part of deleting a user —
see docs/rbac.md ("Group Admin Succession") and docs/api.md (`POST
/admin/users/:id/delete`) — and granting a user the System Admin role, see
docs/api.md (`POST /admin/users/:id/promote`).

Fields:

- \_id
- action (succession | group_auto_deleted | promotion)
- group_id (nullable; set only when action = succession or group_auto_deleted)
- group_name (snapshot — see note below; set only when action = succession or group_auto_deleted)
- deleted_user_id (nullable; the user being deleted, was sole Group Admin of group_id; set only when action = succession or group_auto_deleted)
- deleted_user_name (snapshot; set only when action = succession or group_auto_deleted)
- successor_user_id (nullable; set only when action = succession)
- successor_user_name (nullable; snapshot, set only when action = succession)
- target_user_id (nullable; the user granted System Admin; set only when action = promotion)
- target_user_name (nullable; snapshot, set only when action = promotion)
- performed_by (System Admin's user_id)
- performed_by_name (snapshot)
- created_at

Fields are action-specific: each entry populates only the fields relevant to
its action (see docs/api.md, `GET /admin/audit-log`), so most of the
identity/name fields above are optional on the document. `performed_by` and
`created_at` are the only fields every action shares.

The `*_name` fields are denormalized snapshots captured at write time, not
lookups. By the time the log is read the deleted user (action = succession or
group_auto_deleted) and an auto-deleted group (action = group_auto_deleted) no
longer exist, so their ids can't be resolved to names after the fact — the name
is stored alongside the id when the entry is written. The promoted user (action
= promotion) still exists, but its name is snapshotted too, for the same
reason every other name here is: consistency, and so a later rename doesn't
rewrite history.

Like refresh_tokens, this is system-level data tied to an admin action, not group-scoped tenant data — it is written by System Admin, not queried by group-scoped business logic.

---

# Relationship Model (Important)

- users → refresh_tokens (1-to-many)
- users ↔ groups → many-to-many via group_members
- groups → tickets (1-to-many)
- tickets → comments (1-to-many)
- tickets → ticket_activity (1-to-many)
- tickets ↔ tickets → many-to-many via ticket_links
- tickets → ticket_references (1-to-many)
- tickets → ai_ticket_insights (1-to-1 or 1-to-many over time)
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
- promote another user to System Admin (see docs/api.md, `POST /admin/users/:id/promote`) — one-way, audit-logged; there is no revoke/demote endpoint

System Admin limitations:

- cannot access group tickets unless member
- cannot bypass group isolation
- cannot appoint a Group Admin successor on a group's behalf (see docs/rbac.md, "Group Admin Succession")
- cannot promote a user to System Admin except by calling the promote endpoint as an existing System Admin — the very first System Admin has no such caller, so is set directly against the database, outside the API

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

## ticket_activity

- group_id + ticket_id + occurred_at (compound, descending on occurred_at —
  serves the newest-first feed; its (group_id, ticket_id) prefix also serves
  both cascade deletes)
- group_id + occurred_at (descending — serves find_latest_for_group, the
  group list's `last_activity_at` stat; the compound index above can't
  satisfy this sort since ticket_id sits between the two fields)

## ticket_links

- group_id + source_ticket_id + target_ticket_id + relation_type (compound,
  unique — duplicate-link protection at the DB level, in addition to the
  service-level pre-insert check)
- group_id + target_ticket_id (a ticket can appear on either side of a link;
  this serves the reverse-direction lookup the compound index's prefix can't)

## ticket_references

- group_id + ticket_id

## ai_ticket_insights

- ticket_id
- group_id

## admin_audit_log

- group_id (serves `GET /admin/audit-log?group_id=`)
- deleted_user_id (serves `GET /admin/audit-log?user_id=`)

Separate single-field indexes: the two filters are independent and either may
be used alone.

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
