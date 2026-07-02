# RESOLVE API Specification

---

# Base URL

/api/v1

All endpoint paths below are relative to this base URL.

---

# Authentication

All protected endpoints require:

Authorization: Bearer <JWT>

A token can become invalid before its own expiry: logging out invalidates every
token previously issued to that user, not just the one used to log out.

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
- System Admin endpoints are system-level only

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

## POST /auth/logout

Invalidates every token previously issued to the current user (not just the
one used to call this endpoint).

Requires JWT.

Request: none

Response: none

---

# Group Endpoints

## POST /groups

Create group (any authenticated user)

The creator automatically becomes that group's Group Admin.

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

Add user to group (Group Admin only)

Request:

- user_id
- role (Contributor | Group Admin)

---

## PATCH /groups/:id/users/:user_id

Update a member's role (Group Admin only)

Request:

- role (Contributor | Group Admin)

Used to promote a Contributor to Group Admin, e.g. to appoint a successor before leaving the group.

---

## DELETE /groups/:id/users/:user_id

Remove user from group, including self-removal/leaving (Group Admin only)

Rejected if the target is the sole Group Admin of the group — a successor must be appointed first via PATCH /groups/:id/users/:user_id, or the group must be deleted entirely via DELETE /groups/:id.

---

## DELETE /groups/:id

Delete the group entirely (Group Admin only)

Bypasses the "at least one Group Admin" requirement — the group and all its data cease to exist.

---

## GET /groups/:id

Get group metadata

- System Admin: can see all groups (metadata only)
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

Create ticket (any group member)

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
- Group Admin: all tickets in the group

---

## DELETE /tickets/:id

Group Admin only

---

## POST /tickets/:id/assign

Assign ticket (Group Admin only)

Request:

- user_id

---

## POST /tickets/:id/status

Change status

Group Admin only

---

### Ticket Rules:

- Contributor can only modify own tickets
- Group Admin can modify any ticket in the group
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

Group Admin only

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

# System Admin Endpoints

## GET /admin/groups

List all groups (metadata only)

No ticket access

---

## GET /admin/users

List users

---

## DELETE /admin/users/:id

Delete user

Rejected (409) if the user is the sole Group Admin of any group. That group's succession must be resolved first — an existing member appoints a new Group Admin via PATCH /groups/:id/users/:user_id, or deletes the group via DELETE /groups/:id — using normal group-scoped endpoints. System Admin cannot resolve this on the group's behalf.

---

## DELETE /admin/groups/:id

Delete group

---

## GET /admin/analytics

System-level metrics (aggregated only)

---

### System Admin restrictions:

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
