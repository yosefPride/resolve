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

- Managers create and manage groups
- Contributors operate within assigned groups
- Admin can view system metadata and group lists
- Admin cannot access ticket data unless they are a member of that group

---

## RBAC System

Roles:

- Contributor
- Manager
- Administrator

All permissions enforced on backend only.

---

## RBAC Scope Model

The system has two independent RBAC layers:

- Global role (Admin only)
- Group role (Contributor / Manager per group)

These do NOT override each other.

---

## Scope Rules Clarification

- Admin = system-level operations and metadata access only
- Group roles = all ticket, comment, and workflow operations
- No role bypasses group isolation rules

---

## Admin Data Access Rule

Admin can access:

- user list
- group list (metadata only)

Admin cannot access:

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
