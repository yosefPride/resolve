# RESOLVE API Specification

---

# Base URL

/api/v1

All endpoint paths below are relative to this base URL.

---

# Authentication

All protected endpoints require:

Authorization: Bearer <JWT>

The access token (the JWT above) is short-lived (15 minutes) and verified statelessly — no database lookup, no revocation check. Session continuity and revocation instead live in a separate refresh token, delivered as an httpOnly, SameSite=Strict cookie (never in a JSON body, never readable by JS). See POST /auth/refresh.

The cookie's Secure attribute is environment-dependent: on by default, and required in production, but disabled via config for local HTTP development where a real browser would otherwise refuse to store it. Consuming the cookie cross-origin also requires the API to be configured with the frontend's exact origin (no wildcard) and credentials support enabled.

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

Also sets a refresh_token cookie (httpOnly, SameSite=Strict, Secure per environment config, scoped to /auth). Not part of the JSON body.

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

Response, per group (list-view shape, distinct from the plain group metadata returned by create/get/rename below):

- id, name, created_at
- role — caller's role in that group
- member_count

---

## GET /groups/:id/users/lookup

Look up a user by exact email, to get the `user_id` needed for `POST /groups/:id/users` below (Group Admin only).

There is no join flow and no user directory: the only way into a group is being added by that group's Group Admin, and the only way for them to find the right account is knowing the person's exact email.

Request (query params):

- email (required, exact match, case-sensitive)

Response `200`:

- id
- name
- email

`404` if no user matches. `400` if `email` is missing or empty.

---

## POST /groups/:id/users

Add user to group (Group Admin only)

Request:

- user_id (obtained via GET /groups/:id/users/lookup)
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

## GET /admin/users/:id/deletion-check

Preview what deleting this user would require, before committing to it.

For every group where the user is the sole Group Admin:

- if the group has other members, returns that group's id plus the list of eligible successors (existing members) to choose from
- if the user is that group's only member, returns that group flagged for automatic deletion (no successor possible)

Groups where the user is a Contributor, or a non-sole Group Admin, are not blocking and are omitted — deletion just removes that membership.

Response:

- blocked_groups[] (group_id, eligible_successors[])
- auto_delete_groups[] (group_id)

---

## POST /admin/users/:id/delete

Delete the user, resolving group-admin succession as needed.

Request:

- successors: { group_id: successor_user_id, ... } — required for every group returned in `blocked_groups` by the deletion-check above

Server re-validates at commit time (a successor must still be a member of that group; a group's blocking status may have changed since the check call) rather than trusting the check response blindly. Rejected (409) if a required successor is missing or no longer valid — no partial deletion.

Per group, as part of the same operation:

- sole Group Admin, other members exist → named successor is promoted to Group Admin, then the deleted user's membership is removed
- sole Group Admin, no other members → the group is deleted entirely (cascades its group_members rows)
- non-sole Group Admin or Contributor → membership is simply removed

Every succession or auto-deletion performed this way is recorded in `admin_audit_log` (see docs/database.md).

---

## DELETE /admin/groups/:id

Delete the group entirely — System Admin only.

No membership or succession check: unlike deleting a user, deleting the whole group removes the "at least one Group Admin" requirement along with it, since the group and all its data cease to exist. Group Admins deleting their own group use `DELETE /groups/:id` instead (see Group Endpoints above) — that endpoint remains Group-Admin-scoped and unaffected by this one.

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
