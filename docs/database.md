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
- global_role (Admin only)
- created_at

---

## groups

Represents tenant isolation boundary.

Fields:

- \_id
- name
- owner_id (user who created group)
- created_at

---

## group_members

Defines RBAC inside a group.

Fields:

- \_id
- group_id
- user_id
- role (Contributor | Manager | Admin)
- joined_at

---

## tickets

Core entity of the system.

Fields:

- \_id
- group_id
- title
- description
- status (open | in_progress | closed)
- priority (low | medium | high)
- created_by
- assigned_to
- created_at
- updated_at

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

Stores aggregated AI reports (Manager/Admin only).

Fields:

- \_id
- group_id
- report_data
- generated_at
- generated_by (user_id)

---

# Relationship Model (Important)

- users ↔ groups → many-to-many via group_members
- groups → tickets (1-to-many)
- tickets → comments (1-to-many)
- tickets → ai_ticket_insights (1-to-1 or 1-to-many over time)
- groups → ai_group_reports (1-to-many)

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
- Manager
- Admin

Rules:

- Role is per-group (not global)
- A user can have different roles in different groups

---

# Admin Model

Admin is GLOBAL role stored in users.global_role

Admin capabilities:

- manage users
- manage groups
- view system metadata

Admin limitations:

- cannot access group tickets unless member
- cannot bypass group isolation

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

## groups

- owner_id

## group_members

- group_id + user_id (compound index)

## tickets

- group_id (critical)
- group_id + status
- group_id + assigned_to

## comments

- ticket_id
- group_id

## ai_ticket_insights

- ticket_id
- group_id

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
