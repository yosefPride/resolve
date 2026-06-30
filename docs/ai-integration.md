# AI Integration

---

# Status

Core system feature (not optional)

---

# Role in System

AI is a read-only analysis service for tickets and groups.

It enhances the system but does not control any business logic.

---

# Capabilities

- Ticket summarization
- Severity prediction
- Suggested fixes
- Bug grouping (light clustering within a group)
- Group analytics reports

---

# Execution Model

AI runs in two modes:

## 1. Sync (real-time)

- ticket summarization
- ticket analysis
- fix suggestions

Triggered by user actions or ticket creation.

## 2. Async (background)

- group analytics reports

Generated periodically or on request.

---

# Caching Strategy

- AI results are cached per ticket
- If ticket does not change, AI is not re-run
- Group reports are cached for a period of time

---

# Scope Rules

AI operates strictly within:

- a single group context
- active user permissions (RBAC enforced)

AI NEVER:

- accesses multiple groups (except admin metadata analytics)
- writes to database
- bypasses authorization

---

# Data Flow

1. User opens ticket or triggers AI action
2. Backend validates JWT + group + RBAC
3. Ticket data is sent to AI service
4. AI returns analysis
5. Result is stored (cached) and returned to frontend

---

# Clustering Definition

Bug clustering = grouping similar tickets within the same group based on AI-generated summaries and tags.

(No external ML systems or vector databases required.)

---

# Admin Analytics Rule

Admin can request AI-generated summaries of group-level statistics.

This is aggregated data only and does NOT expose raw cross-group ticket content.

---

# Design Principle

AI is an assistant system, not an autonomous system.
