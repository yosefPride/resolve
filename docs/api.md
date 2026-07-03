# RESOLVE API Specification

---

# Base URL

/api/v1

All endpoint paths below are relative to this base URL.

---

# Authentication

All protected endpoints require:

Authorization: Bearer <JWT>

The access token (the JWT above) is short-lived (15 minutes) and verified statelessly — no database lookup, no revocation check. Session continuity and revocation instead live in a separate refresh token, delivered as an httpOnly, Secure, SameSite=Strict cookie (never in a JSON body, never readable by JS). See POST /auth/refresh.

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

Also sets a refresh_token cookie (httpOnly, Secure, SameSite=Strict, scoped to /auth). Not part of the JSON body.

---

## POST /auth/login

Request:

- email
- password

Response:

- jwt
- user

Also sets a refresh_token cookie, same as register.

---

## GET /auth/me

Returns current user.

Requires JWT.

---

## POST /auth/refresh

Exchanges the refresh_token cookie for a new access token.

Requires: refresh_token cookie (no Authorization header needed — the access token may already be expired by the time a client refreshes).

Response:

- jwt

Also rotates the refresh_token cookie to a new value. Each refresh token is single-use: the presented token is revoked as part of the exchange, so replaying it afterward fails.

Rejected (401) if the cookie is missing, unrecognized, expired, or already used.

---

## POST /auth/logout

Revokes the current session's refresh token (the one in the refresh_token cookie) and clears the cookie.

Requires: refresh_token cookie. Does not require a valid access token.

Per-device only — other sessions/devices for the same user are unaffected. Does not invalidate an access token already issued for this session; that token remains valid until its own (15 minute) expiry.

A request with no refresh_token cookie is a no-op (200), not an error.

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
