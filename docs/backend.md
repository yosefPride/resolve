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

- Static role system (Contributor / Manager / Admin)
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

Group owner = creator (Manager or Admin)

---

# Admin Capabilities

Admin can:

- view all groups (metadata only)
- view users
- delete users
- delete groups
- view system analytics (aggregated only)

Admin cannot:

- Read tickets across groups
- Access private group data without membership
