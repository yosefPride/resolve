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

# Group Scope

There is NO "active group". Scope is carried explicitly by the request path:
every group-scoped resource lives under `/groups/{id}/...`, and the group id in
that path is the only group the request operates on.

- The JWT identifies the user only — it carries no group and no group role
- Membership and role are resolved per request, from the path's group id, by
  the `GroupScoped` extractor (see docs/rbac.md, "Enforcement Mechanism")
- A caller who is not a member of the named group gets 403; there is no way to
  reach one group's data through another group's id
- Because role is looked up per request (never baked into the token), a
  removed or demoted member loses access on their very next request

---

# Core Rules

- No cross-group data access is allowed
- RBAC is enforced server-side only
- AI operates only within the group named in the request path
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

## PATCH /auth/me

Update the caller's own profile. Requires JWT.

Request (both fields optional; at least one required):

- name
- email
- current_password — required only when `email` differs from the current one

Response:

- user (the updated UserResponse)

Changing the email requires `current_password` because the email is the login identity (and the key Group Admins add members by). A name-only change does not. Rejected `400` if the body changes nothing, if a supplied name/email is blank/malformed, or if an email change omits `current_password`; `401` if `current_password` is wrong; `409` (`duplicate_email`) if the email is already in use by another account.

---

## POST /auth/me/password

Change the caller's own password. Requires JWT.

Request:

- current_password
- new_password (minimum 8 characters)

Response: `200` with no body.

On success, every *other* outstanding refresh token for the user is revoked — all other devices are signed out — while the session that made the change (identified by its own refresh_token cookie) stays valid. Rejected `400` if `new_password` is shorter than 8 characters or `current_password` is empty; `401` if `current_password` is wrong.

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
- open_ticket_count — number of tickets in the group with status `open`
- last_activity_at — the `occurred_at` of the group's most recent activity
  entry (see "Activity Endpoint"), across every ticket in the group; `null`
  for a brand-new group with no tickets yet

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

Remove user from group. Removing another member requires Group Admin;
removing yourself (leaving the group) is open to any member.

Rejected if the target is the sole Group Admin of the group — a successor must be appointed first via PATCH /groups/:id/users/:user_id, or the group must be deleted entirely via DELETE /groups/:id.

---

## DELETE /groups/:id

Delete the group entirely (Group Admin only)

Bypasses the "at least one Group Admin" requirement — the group and all its data cease to exist.

---

## GET /groups/:id

Get group metadata (members of the group only)

System Admin sees group metadata through `GET /admin/groups` instead — this
endpoint is member-scoped like every other `/groups/{id}` route.

---

# Ticket Endpoints

All ticket paths are nested under the group they belong to (`/groups/{id}/...`),
so the group scope is always explicit in the URL. `{id}` is the group id;
`{ticket_id}` is the ticket. Membership in `{id}` is required for every one of
these (enforced by the `GroupScoped` extractor).

## GET /groups/{id}/tickets

Returns tickets in the group (any group member)

Supports:

- pagination
- search by title (`q`, case-insensitive substring match; falls back to
  typo-tolerant similarity matching when the substring match returns nothing)
- filters: status, priority, creator

---

## POST /groups/{id}/tickets

Create ticket (any group member)

Request:

- title
- description
- priority

Server-assigned on creation, not accepted from the client:

- `status` — always starts `open`
- `ticket_number` — a running number scoped to the group (the first ticket in
  a group is `1`, independent of other groups' numbering)
- `created_by` — the caller

There is no assignment: tickets have no assignee field.

---

## GET /groups/{id}/tickets/{ticket_id}

Get ticket (any group member). A `{ticket_id}` that is not in `{id}` returns
404 — the repository query is filtered by group id, so a mismatched pair simply
finds nothing (this is what keeps one group's ticket ids unreadable through
another group).

---

## PATCH /groups/{id}/tickets/{ticket_id}

Update ticket — Group Admin only. This includes status changes (there is no
separate status endpoint): `status`, `title`, `description`, and `priority` are
all edited through this one endpoint.

Not even the ticket's creator may edit it after opening — editing is Group
Admin only, full stop. A Contributor may open tickets but cannot modify one
afterward, including their own.

Request (all fields optional, but at least one required):

- title
- description
- priority
- status

---

## DELETE /groups/{id}/tickets/{ticket_id}

Group Admin only

---

### Ticket Rules:

- Any group member may create a ticket
- Only a Group Admin may edit or delete a ticket — including status changes —
  regardless of who created it
- Status: `open` | `closed`
- Priority: `low` | `high` | `critical`
- No assignment: tickets have no assignee

---

# Comment Endpoints

Comment paths are nested under both the group and the ticket they belong to:
`{id}` is the group id, `{ticket_id}` the ticket. Membership in `{id}` is
required for every one of these (`GroupScoped`), and `{ticket_id}` must
actually belong to `{id}` — a mismatched group/ticket pair returns 404, the
same way `GET /groups/{id}/tickets/{ticket_id}` does. Membership in the
caller's own group is not enough to reach another group's ticket through it.

Comments are threaded: a comment may carry a `parent_comment_id` pointing at
another comment on the same ticket, at any depth (a reply to a reply is fine).
A comment with no parent is top-level on the ticket.

## POST /groups/{id}/tickets/{ticket_id}/comments

Add a comment (any group member — no role split on creating one).

Request:

- content (required, 1–2000 characters)
- parent_comment_id (optional) — the comment being replied to, which must be a
  comment on this same ticket; omit for a top-level comment

Server-assigned, not accepted from the client: the author (the caller),
`is_deleted` (false), and `created_at`.

Response `201`: the created comment (shape below).

Rejected `400` if `content` is blank or longer than 2000 characters (counted in
characters, not bytes, so non-Latin content isn't wrongly rejected), or if
`parent_comment_id` is not a comment on this ticket. `409` if the ticket's
status is `closed`.

---

## GET /groups/{id}/tickets/{ticket_id}/comments

Get the ticket's comments (any group member).

Returns the whole thread in one flat response, oldest-first — no pagination and
no server-side nesting; the reply tree is built client-side from each comment's
`parent_comment_id`. Readable regardless of the ticket's status. Tombstoned
comments (see delete below) are included, so their surviving replies still have
a parent to render against.

Response `200`, per comment:

- id
- group_id
- ticket_id
- parent_comment_id (null for a top-level comment)
- user_id, user_name — the author
- content
- is_deleted
- created_at

---

## DELETE /groups/{id}/tickets/{ticket_id}/comments/{comment_id}

Delete a comment — its author, or a Group Admin of `{id}`, only. Allowed even
when the ticket is closed.

Response `204`, no body. `403` if the caller is neither the author nor a Group
Admin; `404` if the comment is not on that ticket in that group.

What deletion does depends on whether the comment has replies:

- has replies → tombstoned rather than removed: `content` becomes
  `[comment deleted]` and `is_deleted` becomes true, so its replies keep a
  valid parent to point at. It still appears in the list response.
- no replies → removed outright

Comments are also removed when their ticket is deleted, and when their group is
deleted.

---

### Comment Rules:

- Any group member may comment on any ticket in their group
- Only the author or a Group Admin may delete a comment
- Threading is unlimited-depth; a parent must be a comment on the same ticket
- Comments cannot be edited — only added and deleted
- A closed ticket accepts no new comments (409), but its existing thread stays
  fully readable and its comments stay deletable; reopening the ticket allows
  new comments again

---

# Link Endpoints

Link paths are nested under both the group and the ticket they belong to:
`{id}` is the group id, `{ticket_id}` the ticket. Membership in `{id}` is
required for every one of these (`GroupScoped`), and `{ticket_id}` must
actually belong to `{id}` — same cross-group guard as Comment Endpoints.

A link relates two tickets in the same group. It is stored directionally
(`source_ticket_id` → `target_ticket_id`), but every response resolves the
label from the viewpoint of `{ticket_id}` in the path — the caller never has
to work out which side is which.

## POST /groups/{id}/tickets/{ticket_id}/links

Add a link from `{ticket_id}` to another ticket (any group member — no role
split on creating one).

Request:

- target_ticket_id (must be a different ticket in the same group)
- relation_type: `blocks` | `relates_to` | `duplicates`

Response `201`: the created link (shape below), label resolved from
`{ticket_id}`'s viewpoint — always `blocks` / `relates_to` / `duplicates`,
never the inverse, since the request's own ticket is always the source.

Rejected `400` if `target_ticket_id` equals `{ticket_id}` (no self-links) or
doesn't resolve to a ticket in this group. `409` if the same
(source, target, relation_type) link already exists — for `relates_to`
specifically (symmetric), this check also covers the reverse direction, since
a single link document represents the pair either way. `blocks` and
`duplicates` are directional: "A blocks B" and "B blocks A" may coexist as two
separate links.

---

## GET /groups/{id}/tickets/{ticket_id}/links

Get every link touching `{ticket_id}`, on either side, oldest-first (any group
member).

Response `200`, per link:

- id
- group_id
- label — resolved for `{ticket_id}`'s viewpoint: `blocks` | `is_blocked_by` |
  `relates_to` | `duplicates` | `is_duplicated_by`
- other_ticket_id, other_ticket_number, other_ticket_title,
  other_ticket_status, other_ticket_priority — a summary of the ticket on the
  other end, enough to render the row without a second request
- created_by, created_by_name
- created_at

---

## DELETE /groups/{id}/tickets/{ticket_id}/links/{link_id}

Delete a link — its creator, or a Group Admin of `{id}`, only.

Response `204`, no body. `403` if the caller is neither the creator nor a
Group Admin; `404` if the link isn't one of `{ticket_id}`'s (on either side)
in that group.

Links are also removed when either ticket in the pair is deleted, and when
their group is deleted.

---

### Link Rules:

- Any group member may add a link between two tickets in their group
- Only the link's creator or a Group Admin may delete it
- No self-links; no duplicate (source, target, relation_type) — and for
  `relates_to`, no duplicate in the reverse direction either
- Links cannot be edited — only added and deleted

---

# Reference Endpoints

Reference paths are nested the same way as Links: `{id}` is the group id,
`{ticket_id}` the ticket, same membership and cross-group guards.

A reference is an external URL attached to a single ticket — unlike a link, it
doesn't relate two tickets to each other.

## POST /groups/{id}/tickets/{ticket_id}/references

Attach a reference to `{ticket_id}` (any group member).

Request:

- url (required, must start with `http://` or `https://`, max 2000 characters)
- label (optional, max 200 characters) — when blank, defaults to the URL's
  host (e.g. `https://github.com/org/repo/pull/12` → `github.com`)

Response `201`: the created reference (shape below).

Rejected `400` if `url` is blank, doesn't start with `http://`/`https://`, or
either field exceeds its length limit.

---

## GET /groups/{id}/tickets/{ticket_id}/references

Get `{ticket_id}`'s references, oldest-first (any group member).

Response `200`, per reference:

- id
- group_id
- ticket_id
- label
- url
- created_by, created_by_name
- created_at

---

## DELETE /groups/{id}/tickets/{ticket_id}/references/{reference_id}

Delete a reference — its creator, or a Group Admin of `{id}`, only.

Response `204`, no body. `403` if the caller is neither the creator nor a
Group Admin; `404` if the reference isn't on that ticket in that group.

References are also removed when their ticket is deleted, and when their
group is deleted.

---

### Reference Rules:

- Any group member may attach a reference to a ticket in their group
- Only the reference's creator or a Group Admin may delete it
- References cannot be edited — only added and deleted

---

# Activity Endpoint

## GET /groups/{id}/tickets/{ticket_id}/activity

Get `{ticket_id}`'s activity feed, newest-first (any group member). Same
membership and cross-group guards as Comment/Link/Reference Endpoints.

This is a read-only history: entries are written exclusively as a side effect
of ticket, comment, link, and reference mutations elsewhere in the API — never
accepted directly from a client.

Response `200`, per entry:

- id
- group_id
- ticket_id
- actor_id, actor_name — who performed the action
- event_type: `ticket_created` | `status_changed` | `priority_changed` |
  `title_changed` | `description_changed` | `comment_added` |
  `comment_deleted` | `link_added` | `link_removed`
- old_value, new_value — populated for `status_changed`, `priority_changed`,
  `title_changed`, `link_added`, `link_removed`; both null for every other
  event type. `description_changed` deliberately carries no text (only the
  fact that it changed) to avoid dumping a full diff into the feed.
- comment_id — populated only for `comment_added`/`comment_deleted`
- link_kind — populated only for `link_added`/`link_removed`: `relation` (a
  Link Endpoint mutation) or `reference` (a Reference Endpoint mutation)
- occurred_at

An update that changes multiple fields in one request (e.g. one PATCH
changing both `status` and `priority`) produces one entry per field that
actually changed — not one combined entry — and only for fields whose value
actually differs from before (re-submitting the same value logs nothing). A
link mutation logs one entry on each side (both tickets in the relation),
each phrased from that ticket's own viewpoint; a reference mutation logs a
single entry, since it only touches one ticket.

---

### Activity Rules:

- Read-only — there is no create/update/delete endpoint; entries are written
  internally by the services above
- Visible to any group member, not just Group Admins — this is history, not
  an admin-only audit trail
- Ticket-level cascade deletes (a whole ticket or group being removed) do not
  themselves log activity — the entries go with them

---

# AI Endpoints (CORE FEATURE)

## POST /ai/groups/{id}/tickets/{ticket_id}/summarize

Returns AI summary of ticket

Group-scoped (member of `{id}` required)

---

## POST /ai/groups/{id}/tickets/{ticket_id}/analyze

Returns:

- severity estimate
- suggested fix
- classification

---

## POST /ai/groups/{id}/report

Group Admin only

Returns:

- group-wide analytics
- ticket trends
- workload distribution

---

### AI Rules:

- All AI results are cached when possible
- AI never modifies database
- AI is scoped to the group named in the request path only

---

# System Admin Endpoints

## GET /admin/groups

List all groups (metadata only)

No ticket access

Optional query param:

- `search` — case-insensitive substring match on the group name. Omitted or blank returns all groups.

---

## GET /admin/users

List users

Optional query param:

- `search` — case-insensitive substring match on user name or email. Omitted or blank returns all users.

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

Every succession or auto-deletion performed this way is recorded in `admin_audit_log` (see docs/database.md), readable via `GET /admin/audit-log` below.

---

## GET /admin/audit-log

Read the succession / auto-deletion audit trail — System Admin only.

Optional query filters, independent (either may be used alone, both may be combined, both may be omitted):

- `group_id` — only entries for that group
- `user_id` — only entries where that user was the one deleted

Entries are returned newest-first. The `*_name` fields are snapshots taken when
the entry was written (the deleted user, and an auto-deleted group, no longer
exist to be looked up at read time). Each entry:

- id
- action (succession | group_auto_deleted)
- group_id
- group_name
- deleted_user_id
- deleted_user_name
- successor_user_id (null when action = group_auto_deleted)
- successor_user_name (null when action = group_auto_deleted)
- performed_by (the System Admin who ran the deletion)
- performed_by_name
- created_at

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
- 204 No Content
- 400 Bad Request
- 401 Unauthorized
- 403 Forbidden
- 404 Not Found
- 409 Conflict (duplicate email, duplicate group member, sole-Group-Admin succession conflicts, commenting on a closed ticket)
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
