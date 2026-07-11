# System Architecture

---

# Overview

RESOLVE is a multi-tenant bug tracking system with strict group isolation and RBAC.

---

# High-Level Flow

Frontend (React JS)
↓
Actix-web REST API
↓
Auth middleware (JWT + RBAC + group check)
↓
Service Layer (Business Logic)
↓
MongoDB + AI (Gemini) service

---

# Core System Principles

## 1. Multi-Tenancy

- Groups are strict isolation boundaries
- Every ticket and comment belongs to exactly one group
- No query may return data outside user's group scope
- Group ID is mandatory in all business entities

---

## 2. RBAC Enforcement

Every request passes:

1. Authentication (JWT)
2. Group membership validation
3. Role-based permission check
4. Business logic execution

All checks happen in backend middleware and service layer.

---

## 3. AI Integration (Core System Feature)

AI is used for:

- Ticket summarization
- Bug classification
- Severity prediction
- Suggested fixes
- Group-level AI reports

AI rules:

- never writes to database
- never bypasses RBAC
- only operates inside a single group context

---

## 4. Data Flow

User → Frontend → API → Auth Middleware → Service → DB / AI → Response

---

# Key Constraint

Group isolation is absolute.

System Admin exceptions apply only to:

- system metadata
- aggregated analytics
- resolving group-admin succession strictly as part of deleting that user (audit-logged; see docs/rbac.md, "Group Admin Succession")

Never raw cross-group ticket access.
