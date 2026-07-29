# Deviations: Code vs. Specification

Every mismatch found while reading `backend/src/` and `frontend/src/` against
`docs/specification/*` and `CLAUDE.md`. Ordered by severity.

Legend:
- **Bug** — the code is wrong or does something harmful.
- **Gap** — specified, simply not built yet.
- **Doc drift** — the code is fine; the spec describes something else.

This file lists **open** mismatches only. Once something is built or corrected it stops being
a deviation — it's described as a normal part of the system in the relevant implementation
doc instead, its entry is removed here, and the remainder are renumbered.

Re-check an entry against the tree before relying on it; the code moves faster than this
document does.

---

## 1. System Admin can delete their own account — **Bug**

**Spec** — `docs/specification/frontend.md` implies the admin's own row is special-cased,
and the frontend comment in `features/users/UserTable.jsx` states it outright:

```js
// The caller's own row has no delete action (backend rejects self-deletion anyway).
```

**Code** — `AdminService::delete_user` never compares `caller_id` to `target_user_id`. There
is no self-deletion guard anywhere in the backend, and no test covers it.

**Consequence** — the UI hides the button, but `POST /admin/users/{own_id}/delete` succeeds.
If that admin is the only System Admin, the system is left with no admin at all and no way
to create one (nothing sets `global_role`; see #2).

The inline comment asserting the backend rejects this is factually wrong and should be
corrected either way.

---

## 2. No way to create a System Admin — **Gap**

**Spec** — `docs/specification/database.md` documents `users.global_role` and a full System
Admin capability set.

**Code** — `UserRepository::create` hardcodes `global_role: None`. No endpoint, service
method, CLI command, or seed script ever sets it. `UserRepository` has no
`update_global_role`.

**Consequence** — the entire `/admin` surface is unreachable on a fresh deployment until
someone edits the `users` collection by hand in a Mongo shell. Worth stating explicitly when
explaining the project — it's a bootstrap gap, not an oversight in the admin module itself.

---

## 3. AI is entirely unimplemented — **Gap**

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

## 4. `GET /admin/analytics` doesn't exist — **Gap**

**Spec** — `docs/specification/api.md` lists it under System Admin Endpoints; `backend.md`
lists "view system analytics (aggregated only)" as a capability.

**Code** — not in `routes.rs`, no handler, no service method.

---

## 5. "EVERY database query MUST include group_id" is not literally true — **Doc drift**

**Spec** — `docs/specification/backend.md`: *"EVERY database query MUST include group_id
filter. No exceptions."* `database.md` repeats it as the "Multi-Tenancy Rule (CRITICAL)".

**Code** — many queries legitimately don't, and cannot:
- `users` — by `_id` or `email`; the admin list has no filter at all
- `refresh_tokens` — by `token_hash` or `user_id`
- `groups` — by `_id`; the admin list has no filter
- `admin_audit_log` — filters are optional; unfiltered returns everything

The actual rule the code follows is narrower and correct: **tenant data** (`tickets`,
`comments`, `group_members`, `counters`) is always group-filtered; **non-tenant data** (users,
sessions, group metadata, system audit) is not. The spec's absolute phrasing would flag
correct code as a violation.

One documented exception inside tenant data: `CommentRepository::has_replies` filters on
`parent_comment_id` alone, and is only ever reached through a comment already fetched with a
group-filtered query.

---

## 6. `database.md` documents the pre-threading `comments` shape — **Doc drift**

**Spec** — `docs/specification/database.md`, `comments`: `_id, group_id, ticket_id, user_id,
content, created_at`, plus two single-field indexes (`ticket_id`, `group_id`).

**Code** — `comment::models::Comment` stores two fields the spec doesn't mention, and both
are load-bearing:

- `parent_comment_id: Option<ObjectId>` — self-referential, nullable, no depth limit. The entire threading feature hangs off it.
- `is_deleted: bool` — marks a tombstone, which is what a comment becomes when it's deleted while it still has replies.

The indexes differ too. `db::ensure_indexes` creates one **compound** `(group_id, ticket_id)`
— serving both the per-ticket read and, through its `group_id` prefix, the group-deletion
cascade — plus `parent_comment_id`, which serves `has_replies`. Neither single-field index
the spec lists is created.

`docs/specification/api.md` was brought in line when the feature was built; `database.md`
wasn't. The code is the sound half here — the fix is to update the spec. See
[`db/collections.md`](./db/collections.md) and [`db/indexes.md`](./db/indexes.md) for the
actual shape.

---

## 7. `GET /groups/:id/users/lookup` is Group-Admin-only, spec is ambiguous — **Doc drift**

**Spec** — `docs/specification/api.md` says "(Group Admin only)" in the prose, but places the
endpoint outside the ticket/member sections where role requirements are listed structurally.

**Code** — `GroupService::lookup_user_by_email` calls `require_group_admin` first. Behavior
matches the prose; only the document's organization is inconsistent. Noted because it's the
kind of thing that reads as a discrepancy on a quick scan.

---

## 8. Small behavioral rough edges — **Bug (minor)**

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

## 9. Unused dependency — **Doc drift (trivial)**

`backend/Cargo.toml` declares `uuid = { version = "1", features = ["v4", "serde"] }`.
No `use uuid` anywhere in `src/`. Every identifier is a Mongo `ObjectId`. Leftover from
an earlier design; safe to remove.

---

## Summary table

| # | Issue | Type | Severity |
|---|---|---|---|
| 1 | Admin can delete own account; UI comment claims otherwise | Bug | **High** |
| 2 | No way to create a System Admin | Gap | **High** |
| 3 | AI unimplemented (declared "core") | Gap | Medium |
| 4 | `GET /admin/analytics` missing | Gap | Low |
| 5 | "every query needs group_id" is overstated | Doc drift | Low |
| 6 | `database.md` missing `parent_comment_id` / `is_deleted`; wrong comment indexes | Doc drift | Low |
| 7 | Lookup endpoint role requirement placement | Doc drift | Trivial |
| 8 | Assorted rough edges (a–g) | Bug (minor) | Low |
| 9 | Unused `uuid` dependency | Doc drift | Trivial |

**The pattern worth noting:** where the code exists, it is careful, well-commented, and
consistent with the spec — the backend's session model, isolation, and succession logic all
do exactly what's documented. What's left divides into **bootstrap and admin-lifecycle holes**
(#1 and #2, which compound: an admin can delete the last admin account, and nothing can
create a replacement) and **AI, specified in detail and not started** (#3 and #4). Those are
different kinds of problem and are worth separating when explaining the project.
