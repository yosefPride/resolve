# RESOLVE - Claude Context

AI-powered multi-tenant bug tracking system with RBAC and AI augmentation.

---

# Tech Stack

Frontend:

- React
- JavaScript
- Vite
- TailwindCSS

Backend:

- Rust
- Actix-web
- MongoDB
- JWT authentication
- bcrypt password hashing

AI:

- Gemini API (core system feature)

---

# Core System Design

## Multi-Tenancy (Groups)

- Strict group-based isolation (tenant system)
- Every user belongs to one or more groups
- Tickets and comments belong to exactly one group
- No cross-group data access is ever allowed

### Group Rules

- Any authenticated user may create a group; the creator becomes that group's Group Admin
- Group Admins manage their group, including adding Contributors and other Group Admins
- Contributors operate within assigned groups
- A group must always have at least one Group Admin: a Group Admin may not leave or be removed while they are the sole Group Admin — a successor must be appointed first, unless the entire group is deleted
- System Admin can view system metadata and group lists
- System Admin cannot access ticket data unless they are a member of that group
- System Admin may resolve group succession, but only as a side effect of deleting that user: if the user is the sole Group Admin of a group, System Admin must explicitly name a successor from that group's existing members before the deletion proceeds; if the user is that group's only member, the group is deleted automatically instead
- This is a narrow, audit-logged exception — System Admin cannot otherwise change group roles or membership

---

## RBAC System

Roles:

- Contributor (group-scoped)
- Group Admin (group-scoped)
- System Admin (global-scoped)

All permissions enforced on backend only.

---

## RBAC Scope Model

The system has two independent RBAC layers:

- Global role (System Admin only)
- Group role (Contributor / Group Admin per group)

These do NOT override each other.

---

## Scope Rules Clarification

- System Admin = system-level operations and metadata access only
- Group roles = all ticket, comment, and workflow operations
- No role bypasses group isolation rules

---

## System Admin Data Access Rule

System Admin can access:

- user list
- group list (metadata only)

System Admin cannot access:

- tickets
- comments
- group internal data (unless member of that group)

---

## Core Rules

- Backend is the source of truth
- Frontend is UI-only
- AI is an advisory system (not required for correctness)
- All operations are scoped to active group
- RBAC enforced on every request

---

## AI Constraints

- AI results should be cached when possible
- Avoid unnecessary repeated AI calls
- AI must never modify database state
- AI must always respect group boundaries

---

## Required Docs (source of truth)

- docs/architecture.md
- docs/backend.md
- docs/frontend.md
- docs/api.md
- docs/database.md
- docs/rbac.md
- docs/ai-integration.md

---

## Claude Code Behavior Rules

- Always enforce group isolation
- Always validate RBAC before executing actions
- Never allow cross-group queries
- Follow Actix patterns (NOT Axum)
- Do not assume TypeScript anywhere
- Prefer incremental changes
- Ask before large refactors

---

# Build Order Discipline (IMPORTANT)

When implementing this project:

1. Auth system (JWT + bcrypt)
2. Group system (create + membership)
3. RBAC enforcement (middleware)
4. Ticket system (core CRUD)
5. Comments system
6. Frontend integration
7. AI features LAST (after core system is stable)

Do NOT implement AI before core system works.

---

# Design Principle

Keep the system simple, enforceable, and consistent across all layers.
