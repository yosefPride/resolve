# RESOLVE API Specification

---

# Base URL

/api/v1

All endpoint paths below are relative to this base URL.

---

# Authentication

All protected endpoints require:

Authorization: Bearer <JWT>

---

# Group Context

Each user has an ACTIVE GROUP.

- Active group is stored in JWT or user session
- All API requests operate only within active group
- Users may switch active group via dedicated endpoint

---

# Core Rules

- No cross-group data access is allowed
- RBAC is enforced server-side only
- AI operates only within active group
- Admin endpoints are system-level only

---

# Authentication Endpoints

## POST /auth/register

Create user.

Request:

- email
- password
- name

Response:

- user
- jwt

---

## POST /auth/login

Request:

- email
- password

Response:

- jwt
- user

---

## GET /auth/me

Returns current user.

Requires JWT.

---

# Group Endpoints

## POST /groups

Create group (Manager or Admin only)

Request:

- name

Response:

- group

---

## GET /groups

List groups user belongs to

Response:

- groups[]

---

## POST /groups/:id/join

Join group (if allowed or invite-based)

---

## POST /groups/:id/users

Add user to group (Manager/Admin only)

Request:

- user_id
- role

---

## DELETE /groups/:id/users/:user_id

Remove user from group (Manager/Admin only)

---

## GET /groups/:id

Get group metadata

- Admin: can see all groups
- Others: only own groups

---

# Ticket Endpoints

## GET /tickets

Returns tickets in current group

Supports:

- pagination
- filters (status, assignee)

---

## POST /tickets

Create ticket (Contributor+)

Request:

- title
- description
- priority

---

## GET /tickets/:id

Get ticket (must belong to same group)

---

## PATCH /tickets/:id

Update ticket

Permissions:

- Contributor: own tickets only
- Manager/Admin: all tickets

---

## DELETE /tickets/:id

Admin only

---

## POST /tickets/:id/assign

Assign ticket (Manager/Admin only)

Request:

- user_id

---

## POST /tickets/:id/status

Change status

Manager/Admin only

---

### Ticket Rules:

- Contributor can only modify own tickets
- Ownership defined by creator_id

---

# Comment Endpoints

## POST /tickets/:id/comments

Add comment (all roles)

---

## GET /tickets/:id/comments

Get comments (group-scoped)

---

# AI Endpoints (CORE FEATURE)

## POST /ai/tickets/:id/summarize

Returns AI summary of ticket

Group-scoped

---

## POST /ai/tickets/:id/analyze

Returns:

- severity estimate
- suggested fix
- classification

---

## POST /ai/groups/:id/report

Manager/Admin only

Returns:

- group-wide analytics
- ticket trends
- workload distribution

---

### AI Rules:

- All AI results are cached when possible
- AI never modifies database
- AI is scoped to active group only

---

# Admin System Endpoints

## GET /admin/groups

List all groups (metadata only)

No ticket access

---

## GET /admin/users

List users

---

## DELETE /admin/users/:id

Delete user

---

## DELETE /admin/groups/:id

Delete group

---

## GET /admin/analytics

System-level metrics (aggregated only)

---

### Admin restrictions:

- No ticket-level access
- No comment-level access
- Only system metadata + aggregates

---

# Error Format

All errors return:

```json
{
  "error": {
    "code": "string",
    "message": "string"
  }
}
```

---

# Status Codes

- 200 OK
- 201 Created
- 400 Bad Request
- 401 Unauthorized
- 403 Forbidden
- 404 Not Found
- 500 Internal Server Error

---

# Security Rules

- Never expose cross-group data
- Never return raw internal errors
- Always validate JWT first
- Always validate group membership second
- Always enforce RBAC third
- AI never bypasses any layer

---

# Design Principle

The API is **group-first, role-secured, AI-augmented, and strictly isolated by tenant boundaries**.
