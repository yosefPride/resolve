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

---

# Execution Model

AI runs synchronously, in real time:

- ticket summarization
- ticket analysis
- fix suggestions

Triggered by user actions or ticket creation.

---

# Caching Strategy

- AI results are cached per ticket
- If ticket does not change, AI is not re-run

---

# Scope Rules

AI operates strictly within:

- a single group context
- active user permissions (RBAC enforced)

AI NEVER:

- accesses multiple groups (except system admin metadata analytics)
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

# System Admin Analytics Rule

System Admin can request AI-generated summaries of group-level statistics.

This is aggregated data only and does NOT expose raw cross-group ticket content.

---

# Design Principle

AI is an assistant system, not an autonomous system.
