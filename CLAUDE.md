# RESOLVE - Claude Context

AI-powered multi-tenant bug tracking system with RBAC and AI augmentation.

---

# Core System Design

## Multi-Tenancy (Groups)

- Strict group-based isolation (tenant system)
- Every user belongs to one or more groups
- Tickets and comments belong to exactly one group
- No cross-group data access is ever allowed

---

## Core Rules

- Backend is the source of truth
- Frontend is UI-only
- AI is an advisory system (not required for correctness)
- All group operations are scoped to a group named explicitly in the request path (`/groups/{id}/...`); there is no "active group"
- RBAC enforced on every request

---

## AI Constraints

- AI results should be cached when possible
- Avoid unnecessary repeated AI calls
- AI must never modify database state
- AI must always respect group boundaries

---

## Required Docs (source of truth)

Design intent — what the system is supposed to be:

- docs/specification/architecture.md
- docs/specification/backend.md
- docs/specification/frontend.md
- docs/specification/api.md
- docs/specification/database.md
- docs/specification/rbac.md
- docs/specification/ai-integration.md

As built — what the code actually does, and where it diverges from the above:

- docs/implementation/backend-flow.md
- docs/implementation/frontend-flow.md
- docs/implementation/data-model.md
- docs/implementation/deviations.md

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

# Design Principle

Keep the system simple, enforceable, and consistent across all layers.
