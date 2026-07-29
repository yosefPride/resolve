# Deviations: Code vs. Specification

Every mismatch found while reading `backend/src/` and `frontend/src/` against
`docs/specification/*` and `CLAUDE.md`. Ordered by severity.

Legend:
- **Bug** — the code is wrong or does something harmful.
- **Gap** — specified, simply not built yet.
- **Doc drift** — the code is fine; the spec describes something else.
- **Fixed** — resolved since this document was written; the entry is kept for the history.

Three of the twelve (#1, #4, #9) are now fixed. The numbering is stable, so entries are
never renumbered or removed.

---

## 1. Deleting a group orphans its tickets and counter — **Fixed**

Resolved by centralizing the cascade in `purge_group_data` (`group/service.rs`), which all
three deletion paths now call: `GroupService::delete_group`,
`AdminService::delete_group`, and the auto-delete loop in `AdminService::delete_user`.
`TicketRepository::delete_by_group` removes the group's tickets and its `counters`
document; the group document itself is deleted last, so a mid-failure leaves the cascade
re-runnable rather than orphaning what it was meant to remove. Covered by
`test_delete_group_cascades_tickets_and_counter`,
`test_admin_delete_group_cascades_tickets_and_counter`, and
`test_delete_user_auto_delete_cascades_tickets_and_counter`.

Comments were built immediately after, and their per-group cascade
(`CommentRepository::delete_by_group`) went into the same function. The separate
single-ticket cascade lives in `TicketService::delete_ticket` via `delete_by_ticket`.

Original report follows.



**Spec** — `docs/specification/api.md`, `DELETE /groups/:id`: *"the group and all its data
cease to exist."* `docs/specification/database.md`, `counters`: *"Deleted along with the
group's tickets when the group is deleted."*

**Code** — neither deletion path touches tickets or counters:

```rust
// group/service.rs — delete_group
self.repo.delete_members_by_group(group_id).await?;
self.repo.delete_group(group_id).await?;

// admin/service.rs — delete_group
self.group_repo.delete_members_by_group(group_id).await?;
let deleted = self.group_repo.delete_group(group_id).await?;
```

`TicketRepository` has no `delete_by_group` method at all, and nothing deletes from
`counters` anywhere in the codebase.

**Consequences**
- Every ticket of a deleted group stays in the `tickets` collection forever, unreachable (no group means no `GroupScoped` extractor can resolve, so no endpoint can read them) but still stored. Unbounded growth.
- The `counters` document survives. If a group id were ever reused, numbering would resume mid-sequence — not currently possible with `ObjectId`, but the row is still garbage.
- The same gap applies to `AdminService::delete_user`'s auto-delete path, which deletes groups the same way.

This is the most significant deviation in the project. Two obvious fixes: add
`TicketRepository::delete_by_group` + a counter delete and call both from each path, or
accept soft-orphaning and document it.

---

## 2. System Admin can delete their own account — **Bug**

**Spec** — `docs/specification/frontend.md` implies the admin's own row is special-cased,
and the frontend comment in `features/users/UserTable.jsx` states it outright:

```js
// The caller's own row has no delete action (backend rejects self-deletion anyway).
```

**Code** — `AdminService::delete_user` never compares `caller_id` to `target_user_id`. There
is no self-deletion guard anywhere in the backend, and no test covers it.

**Consequence** — the UI hides the button, but `POST /admin/users/{own_id}/delete` succeeds.
If that admin is the only System Admin, the system is left with no admin at all and no way
to create one (nothing sets `global_role`; see #3).

The inline comment asserting the backend rejects this is factually wrong and should be
corrected either way.

---

## 3. No way to create a System Admin — **Gap**

**Spec** — `docs/specification/database.md` documents `users.global_role` and a full System
Admin capability set.

**Code** — `UserRepository::create` hardcodes `global_role: None`. No endpoint, service
method, CLI command, or seed script ever sets it. `UserRepository` has no
`update_global_role`.

**Consequence** — the entire `/admin` surface is unreachable on a fresh deployment until
someone edits the `users` collection by hand in a Mongo shell. Worth stating explicitly when
explaining the project — it's a bootstrap gap, not an oversight in the admin module itself.

---

## 4. Comments are entirely unimplemented — **Fixed (backend); frontend still open**

The backend is built: `comment/` is a full module (models, repository, service, handlers),
three routes are registered, the collection carries two indexes, and both cascades (ticket
deletion, group deletion) are wired in. See
[`backend/07-comments.md`](./backend/07-comments.md). `RbacService::require_owner_or_group_admin`
now has its intended caller (see #9).

The implementation went **beyond** the spec in three ways, all of which have since been
written back into `docs/specification/api.md`:

- **Threading.** The spec's `comments` shape has no parent field; the code adds `parent_comment_id`, self-referential with no depth limit.
- **Deletion.** The spec defines no delete endpoint at all. The code has one (author or Group Admin), plus `is_deleted` and a tombstone rule for comments that have replies — neither field is in the spec's schema.
- **The closed-ticket lock.** Not specified anywhere; a closed ticket rejects new comments with `409` while staying readable.

`docs/specification/database.md` still lists the pre-threading `comments` shape
(`_id, group_id, ticket_id, user_id, content, created_at`) and two single-key indexes. The
code stores `parent_comment_id` and `is_deleted` as well, and creates one compound
`(group_id, ticket_id)` index plus a `parent_comment_id` index. **That schema doc has not
been updated** — see `db/collections.md` and `db/indexes.md` for what actually exists.

**Still open:** the frontend. `features/comments/CommentForm.jsx`, `CommentList.jsx`, and
`hooks/useComments.js` remain empty files — there is no comment UI, which is part of #6.

---

## 5. AI is entirely unimplemented — **Gap**

**Spec** — `CLAUDE.md` calls the Gemini API *"a core system feature"*.
`docs/specification/api.md` defines three AI endpoints; `database.md` defines
`ai_ticket_insights` and `ai_group_reports`; `ai-integration.md` is a 100-line document.

**Code** — all five files in `backend/src/ai/` are **0 bytes**. `routes.rs` registers
`web::scope("/ai")` with **no routes inside it**. `Cargo.toml` has no Gemini SDK and no HTTP
client (`reqwest`) — so there isn't even a dependency in place. Frontend `features/ai/*` (3
files), `hooks/useAI.js`, and `services/ai.service.js` are all empty.

Consistent with `CLAUDE.md`'s own build order ("Do NOT implement AI before core system
works" — step 7 of 7), so this is planned sequencing rather than drift. But "core feature"
overstates what exists.

---

## 6. The frontend has no ticket UI, and the nav link 404s — **Gap + Bug**

**Spec** — `docs/specification/frontend.md`: *"the Tickets page lists all the groups the user
belongs to, and selecting one loads that group's tickets"*, plus a Ticket Detail page with
comments and an AI panel.

**Code** — backend tickets **and comments** are now fully implemented, but the frontend has
nothing for either:

```
pages/TicketsPage.jsx           0 bytes
pages/TicketDetailPage.jsx      0 bytes
features/tickets/*  (5 files)   0 bytes
features/comments/* (2 files)   0 bytes
hooks/useTickets.js             0 bytes
hooks/useComments.js            0 bytes
services/tickets.service.js     0 bytes
services/comments.service.js    0 bytes
```

The gap widened with the comments backend: the Ticket Detail page the spec describes now has
a complete API behind it (including the reply tree, which the client is expected to assemble
from the flat `parent_comment_id` list) and still no page to render it.

**The bug on top of the gap:** both `components/layout/Header.jsx` and
`components/layout/Sidebar.jsx` include a nav link to `/tickets`, but `App.jsx` registers no
such route. Clicking "Issues" — visible in both chromes, on every authenticated page — falls
through to the `*` catch-all and renders `NotFoundPage`.

Either the route should be added or the link should be made inert (the way the Notifications
row already is, with a "soon" badge — that pattern is right there in `Sidebar.jsx`).

---

## 7. `GET /admin/analytics` doesn't exist — **Gap**

**Spec** — `docs/specification/api.md` lists it under System Admin Endpoints; `backend.md`
lists "view system analytics (aggregated only)" as a capability.

**Code** — not in `routes.rs`, no handler, no service method.

---

## 8. "EVERY database query MUST include group_id" is not literally true — **Doc drift**

**Spec** — `docs/specification/backend.md`: *"EVERY database query MUST include group_id
filter. No exceptions."* `database.md` repeats it as the "Multi-Tenancy Rule (CRITICAL)".

**Code** — many queries legitimately don't, and cannot:
- `users` — by `_id` or `email`; the admin list has no filter at all
- `refresh_tokens` — by `token_hash` or `user_id`
- `groups` — by `_id`; the admin list has no filter
- `admin_audit_log` — filters are optional; unfiltered returns everything

The actual rule the code follows is narrower and correct: **tenant data** (`tickets`,
`group_members`, `counters`) is always group-filtered; **non-tenant data** (users, sessions,
group metadata, system audit) is not. The spec's absolute phrasing would flag correct code as
a violation.

---

## 9. `require_owner_or_group_admin` is documented as active but is dead code — **Fixed**

**Was** — written, called by nothing. Tickets deliberately use `require_group_admin`
instead, and the only other intended consumer — comments — didn't exist.

**Now** — `CommentService::delete_comment` calls it, which is exactly the "comment rule"
`docs/specification/rbac.md` describes. It remains the **only** caller: authorship grants a
permission nowhere else in the system, and tickets still give their creator nothing.

---

## 10. `GET /groups/:id/users/lookup` is Group-Admin-only, spec is ambiguous — **Doc drift**

**Spec** — `docs/specification/api.md` says "(Group Admin only)" in the prose, but places the
endpoint outside the ticket/member sections where role requirements are listed structurally.

**Code** — `GroupService::lookup_user_by_email` calls `require_group_admin` first. Behavior
matches the prose; only the document's organization is inconsistent. Noted because it's the
kind of thing that reads as a discrepancy on a quick scan.

---

## 11. Small behavioral rough edges — **Bug (minor)**

Found in code, not contradicted by any spec, but worth knowing:

**a. Setting a member's current role returns 404.**
`GroupRepository::update_member_role` returns `modified_count > 0`, and
`GroupService::update_member_role` maps `false` → `NotFound`. Promoting an existing Group
Admin to Group Admin therefore 404s rather than being an idempotent no-op.

**b. `add_member` doesn't verify the target user exists.**
No `find_by_id` on `target_user_id`. In practice the id comes from
`lookup_user_by_email`, but a crafted request can insert a membership row pointing at
nothing. `enrich_member` then renders empty name/email via `unwrap_or_default()`.

**c. Ticket title length is measured in bytes.**
`handlers.rs` checks `input.title.len() > MAX_TITLE_LEN` — `String::len()` is bytes, not
chars. A 200-character title in a non-Latin script is rejected. `title.chars().count()`
would match intent. (Note `levenshtein_distance` in `utils/` *does* handle this correctly
via `Vec<char>`, so the codebase is inconsistent with itself here.)

**d. Group name has no length limit.**
`validate_name` only rejects blank. Ticket titles are capped at 200; group names aren't
capped at all.

**e. Audit entries are written after the writes they describe.**
In `AdminService::delete_user`, the role change and membership removal happen before
`insert_audit_entry`. A crash between them loses the log line for a change that did occur.
Given the no-transaction design this is a deliberate simplification, but it means the audit
log is not a guaranteed-complete record.

**f. `DELETE /admin/groups/:id` is not audit-logged.**
Only succession and auto-deletion write entries. A System Admin deleting a group outright
leaves no trail — `docs/specification/rbac.md` does state this explicitly, so it's intended,
but it's a real gap in the audit story.

**g. Nothing invalidates `['admin', 'auditLog']`.**
Deleting a user writes audit entries, but no frontend code invalidates that query key. The
admin must switch tabs to see them. Self-correcting in practice (tab switching remounts the
panel).

---

## 12. Unused dependency — **Doc drift (trivial)**

`backend/Cargo.toml` declares `uuid = { version = "1", features = ["v4", "serde"] }`.
No `use uuid` anywhere in `src/`. Every identifier is a Mongo `ObjectId`. Leftover from
an earlier design; safe to remove.

---

## Summary table

| # | Issue | Type | Severity |
|---|---|---|---|
| 1 | ~~Group deletion orphans tickets + counters~~ | Bug | **Fixed** |
| 2 | Admin can delete own account; UI comment claims otherwise | Bug | **High** |
| 3 | No way to create a System Admin | Gap | **High** |
| 4 | ~~Comments unimplemented~~ (backend done; no UI, see #6) | Gap | **Fixed** |
| 5 | AI unimplemented (declared "core") | Gap | Medium |
| 6 | No ticket UI; `/tickets` nav link 404s | Gap + Bug | Medium |
| 7 | `GET /admin/analytics` missing | Gap | Low |
| 8 | "every query needs group_id" is overstated | Doc drift | Low |
| 9 | ~~`require_owner_or_group_admin` is dead code~~ | Doc drift | **Fixed** |
| 10 | Lookup endpoint role requirement placement | Doc drift | Trivial |
| 11 | Assorted rough edges (a–g) | Bug (minor) | Low |
| 12 | Unused `uuid` dependency | Doc drift | Trivial |

**The pattern worth noting:** where the code exists, it is careful, well-commented, and
consistent with the spec — the backend's session model, isolation, and succession logic all
do exactly what's documented. The deviations cluster in two places: **cleanup on delete**
(#1, and the reason #2 matters), and **features that were specified before they were built**
(#4–#7). Those are different kinds of problem and are worth separating when explaining the
project.
