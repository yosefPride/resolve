# Backend Architecture

---

# Stack

- Rust
- Actix-web
- MongoDB
- JWT
- bcrypt
- Gemini API

---

# Core Design

The backend follows a simple layered architecture:

Request → Handler → Service → Repository → Database

---

# Core Modules

src/

    admin/
    ai/
    auth/
    comment/
    config.rs
    errors/
    group/
    rbac/
    server/
    state.rs
    ticket/
    user/
    utils/
    main.rs

Each module follows a feature-based architecture.

---

## Module Pattern

Each feature should follow:

- handlers.rs
- service.rs
- repository.rs
- models.rs
- mod.rs

---

## Layer Responsibilities

### Handlers

- HTTP endpoints
- Input validation
- Calls service layer

### Services

- Business logic
- Authorization rules
- AI integration orchestration

### Repositories

- MongoDB access only
- No business logic allowed
- MUST include group_id filter

---

# Active Group Concept

- Every user has an ACTIVE GROUP
- Stored in JWT
- All operations are scoped automatically to this group
- No manual group passing required in business logic

---

# Request Lifecycle

1. JWT validation
2. User extraction
3. Group context resolution
4. RBAC check
5. Handler execution
6. Service logic
7. Repository access (scoped by group)

---

# Group Isolation Rule

EVERY database query MUST include group_id filter.

No exceptions.

---

# RBAC System

- Static role system: Contributor / Group Admin (group-scoped), System Admin (global-scoped)
- Enforced via middleware + helper functions
- No dynamic policy engine

---

# AI Integration

AI is a core feature but treated as a service dependency:

- Ticket summarization (sync)
- Bug classification (sync)
- Group reports (async recommended)

Rules:

- AI never writes to DB
- AI never bypasses RBAC
- AI always scoped to active group

---

# Group Module

Responsible for:

- creating groups
- managing members
- enforcing ownership rules
- enforcing the "at least one Group Admin per group" invariant

Group creation is open to any authenticated user. The creator is automatically assigned the Group Admin role for that group (a group_members row is created in the same operation). A Group Admin may not leave or be removed while they are the group's sole Group Admin — a successor must be appointed first, unless the group is deleted entirely.

---

# System Admin Capabilities

System Admin can:

- view all groups (metadata only)
- view users
- delete users
- delete groups
- view system analytics (aggregated only)

System Admin cannot:

- Read tickets across groups
- Access private group data without membership
- Delete a user who is the sole Group Admin of a group (must be resolved by that group's own members first)
