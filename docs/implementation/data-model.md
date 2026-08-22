# Data model — as built

Every MongoDB collection as it exists in code, with its fields, its indexes, and
the rules that keep it consistent. Field names below are the **stored** names
(what the Rust structs serialize to), not the API response shapes — those differ
in that ids come back as hex strings and timestamps as RFC 3339.

Source of truth: `backend/src/*/models.rs` for documents, `backend/src/db.rs` for
indexes. The intended design lives in
[`docs/specification/database.md`](../specification/database.md).

---

## Conventions

- **Ids.** Every document's primary key is `_id` (`ObjectId`), mapped to a Rust
  `id: Option<ObjectId>` that is skipped on serialize so Mongo assigns it. One
  collection keys on a natural id instead: `counters`, whose `_id` is the group
  id.
- **Timestamps** are BSON `DateTime` (UTC milliseconds), converted to
  `chrono::DateTime<Utc>` at the response boundary.
- **Enums** serialize `snake_case` (`contributor`, `group_admin`, `open`,
  `relates_to`, …). The one exception is `GlobalRole`, which has no rename and
  therefore serializes as `SystemAdmin` — the frontend's `utils/roles.js`
  mirrors this asymmetry deliberately.
- **Group isolation.** Every tenant-scoped collection carries `group_id`, and
  every repository query filters on it. There are no cross-group queries.
- **No schema validation** is configured in MongoDB; shape is enforced entirely
  by serde on read and write. A document that fails to deserialize fails the
  whole query, which is why new non-optional fields need a backfill (see
  *Migrations* below).
- **Referential integrity** is application-level. Nothing cascades in the
  database; deletes are done explicitly by services.

---

## Collections

### `users`

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `email` | String | Login identity; unique. |
| `password_hash` | String | bcrypt. Never leaves the backend. |
| `name` | String | Display name. |
| `global_role` | `"SystemAdmin"` \| null | The entire global RBAC layer. Null = ordinary user. |
| `created_at` | DateTime | |

Indexes: `{ email: 1 }` unique.

The global role is deliberately a nullable single-variant enum rather than a
boolean, so additional global roles can be added without a migration.

### `refresh_tokens`

One document per outstanding refresh token — effectively one per logged-in
device.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `user_id` | ObjectId | |
| `token_hash` | String | SHA-256 of the raw token. The raw value is never stored. |
| `created_at` | DateTime | |
| `expires_at` | DateTime | 30 days out. |
| `revoked_at` | DateTime \| null | Set on rotation (tokens are single-use) or logout. |

Indexes: `{ token_hash: 1 }` unique; `{ expires_at: 1 }` TTL (expire-after 0, so
Mongo's reaper drops rows once `expires_at` passes — no cron job).

Revoked-but-unexpired rows stick around until the TTL catches them; validity
checks read `revoked_at`, so that is harmless.

### `groups`

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | The tenant boundary. |
| `name` | String | Not unique — two teams may share a name. |
| `owner_id` | ObjectId | The creator. Historical only: it confers no permission. |
| `created_at` | DateTime | |

Indexes: none beyond `_id`.

`owner_id` is *not* the authorization path. Permission comes from the caller's
row in `group_members`, so an owner who was later demoted has no admin rights,
and an owner who left the group has none at all.

### `group_members`

The join table that carries the group-scoped RBAC layer.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id` | ObjectId | |
| `user_id` | ObjectId | |
| `role` | `contributor` \| `group_admin` | |
| `joined_at` | DateTime | |

Indexes: `{ group_id: 1, user_id: 1 }` unique — this is the membership lookup on
every group-scoped request, and the constraint behind the duplicate-member
rejection; `{ user_id: 1 }` plain, because "list my groups" filters on `user_id`
alone and the compound index cannot serve it (`user_id` is not its prefix).

**Invariant:** a group always has at least one `group_admin`. Enforced in
`GroupService` (not in RBAC, and not by the database) on member removal, role
change, and leave; the only path around it is System Admin user deletion, which
must name a successor first and writes an audit entry.

### `tickets`

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id` | ObjectId | |
| `ticket_number` | i64 | Per-group running number, the human-facing id. |
| `title` | String | |
| `description` | String | Rendered as Markdown by the client. |
| `status` | `open` \| `closed` | |
| `priority` | `low` \| `high` \| `critical` | |
| `created_by` | ObjectId | |
| `created_at` | DateTime | |
| `updated_at` | DateTime | Bumped by every edit, status included. |
| `content_updated_at` | DateTime | Bumped only by title/description/priority edits. |

Indexes: `{ group_id: 1 }`; `{ group_id: 1, status: 1 }`;
`{ group_id: 1, created_by: 1 }`; `{ group_id: 1, ticket_number: 1 }` unique.

The two update timestamps exist to keep AI caching correct: the model only ever
reads title and description, so closing or reopening a ticket must not throw away
a good cached summary. `content_updated_at` is the cache fingerprint;
`updated_at` is the general "last touched" value.

### `counters`

Backs the per-group ticket sequence. One document per group.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | The group id — the natural key, so no separate index. |
| `ticket_seq` | i64 | Last allocated number. |

Allocated by an atomic `find_one_and_update` with `$inc`, so concurrent creates
cannot collide. The unique index on `(group_id, ticket_number)` is the backstop.
Numbers are never reused: deleting a ticket does not decrement the counter.

### `comments`

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id` | ObjectId | |
| `ticket_id` | ObjectId | |
| `parent_comment_id` | ObjectId \| null | Null for a top-level comment. One level of threading. |
| `user_id` | ObjectId | Author. |
| `content` | String | |
| `is_deleted` | bool | Tombstone flag. |
| `created_at` | DateTime | |

Indexes: `{ group_id: 1, ticket_id: 1 }` (list, plus both cascade deletes, which
use a prefix of it); `{ parent_comment_id: 1 }` (the has-replies check).

**Delete is conditional.** A comment with no replies is hard-deleted. A comment
that has replies is tombstoned instead — `is_deleted` flips to true and `content`
is replaced with `[comment deleted]` — so its replies keep a valid
`parent_comment_id` to point at rather than referencing a vanished id. A
tombstoned comment's reactions are cleared at the same time.

### `comment_reactions`

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id`, `ticket_id`, `comment_id` | ObjectId | Denormalized so cascades filter directly. |
| `user_id` | ObjectId | |
| `emoji` | String | |
| `created_at` | DateTime | |

Indexes: `{ comment_id: 1, user_id: 1 }` unique;
`{ group_id: 1, ticket_id: 1 }` for cascades.

One reaction per user per comment, enforced by the unique index and an upsert on
that key rather than a check-then-write. Picking a different emoji **replaces**
the existing reaction. Clients never see individual rows — the API returns
per-emoji counts plus a `reacted_by_me` flag.

### `ticket_activity`

The per-ticket history. Insert-only, and never written from a client request —
services write entries as a side effect of their own mutations.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id`, `ticket_id` | ObjectId | |
| `actor_id` | ObjectId | Name resolved at read time, so renames are reflected. |
| `event_type` | enum | `ticket_created`, `status_changed`, `priority_changed`, `title_changed`, `description_changed`, `comment_added`, `comment_deleted`, `link_added`, `link_removed`. |
| `old_value`, `new_value` | String \| null | Populated for status/priority/title and link events. |
| `comment_id` | ObjectId \| null | Comment events only. No comment text is ever copied here. |
| `link_kind` | `relation` \| `reference` \| null | Link events only. |
| `occurred_at` | DateTime | |

Indexes: `{ group_id: 1, ticket_id: 1, occurred_at: -1 }` (the per-ticket
newest-first list, plus cascades); `{ group_id: 1, occurred_at: -1 }` (the
group's latest activity — the compound index above cannot serve a sort on
`occurred_at` without pinning `ticket_id`, which sits between them).

**One entry per discrete change.** A single PATCH touching status, priority, and
title writes three rows, so the timeline reads as granular events rather than an
opaque "ticket updated". `description_changed` deliberately stores no before/after
values: a raw text dump is not useful in a timeline.

### `ticket_links`

Typed relations between two tickets in the same group.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id` | ObjectId | Both tickets are in it — links never cross groups. |
| `source_ticket_id`, `target_ticket_id` | ObjectId | Direction as stored. |
| `relation_type` | `blocks` \| `relates_to` \| `duplicates` | |
| `created_by` | ObjectId | |
| `created_at` | DateTime | |

Indexes:
`{ group_id: 1, source_ticket_id: 1, target_ticket_id: 1, relation_type: 1 }`
unique (its `(group_id, source_ticket_id)` prefix also serves list/delete);
`{ group_id: 1, target_ticket_id: 1 }` for the target-side branch, since a ticket
can sit on either end.

**One document per relation, never two.** `relates_to` is symmetric, so the
service checks both directions before inserting. `blocks` and `duplicates` are
directional, and the inverse reading ("is blocked by", "is duplicated by") is
derived at read time from which side the viewing ticket is on — it is not stored.
Deleting a ticket removes links where it appears on either side, so no dangling
half is left behind.

### `ticket_references`

External URLs attached to a ticket (a PR, a doc, a thread) — distinct from
`ticket_links`, which relate two tickets to each other.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id`, `ticket_id` | ObjectId | |
| `label` | String | Always populated: falls back to the URL's host. |
| `url` | String | |
| `created_by` | ObjectId | |
| `created_at` | DateTime | |

Indexes: `{ group_id: 1, ticket_id: 1 }`.

### `ai_ticket_insights`

The AI cache. **One document per ticket**, upserted in place rather than
appended.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id`, `ticket_id` | ObjectId | |
| `summary` | String \| null | |
| `summary_source_updated_at` | DateTime \| null | Snapshot of the ticket's `content_updated_at` when the summary was generated. |
| `severity_prediction`, `suggested_fix`, `classification` | String \| null | The analysis triple. |
| `analysis_source_updated_at` | DateTime \| null | Same snapshot, for the analysis. |
| `created_at`, `updated_at` | DateTime | |

Indexes: `{ group_id: 1, ticket_id: 1 }` unique.

A field group is **fresh** iff its values are present *and* its source timestamp
still equals the ticket's current `content_updated_at`. Summary and analysis
track their fingerprints separately, because they are independent calls — an edit
can invalidate one that has run while the other never has.

### `ai_conversations`

A private, per-user chat thread on a ticket.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `group_id`, `ticket_id` | ObjectId | |
| `user_id` | ObjectId | The owner, fixed at creation. |
| `title` | String \| null | Null until the first message; derived from it, not model-generated. |
| `created_at`, `updated_at` | DateTime | `updated_at` bumps on each message. |

Indexes: `{ group_id: 1, ticket_id: 1, user_id: 1, updated_at: -1 }` — equality
fields first, sort field last; its prefix also covers the cascades.

Ownership here is stricter than anywhere else in the system: no other member, not
even a Group Admin, can list, read, or delete someone else's conversation.

### `ai_chat_messages`

One document per message — a conversation is a sequence, not a latest value.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `conversation_id` | ObjectId | |
| `group_id`, `ticket_id` | ObjectId | Denormalized so cascades delete by them directly. |
| `role` | `user` \| `assistant` | Provider-agnostic by design. |
| `user_id` | ObjectId \| null | Null for assistant messages. |
| `content` | String | |
| `created_at` | DateTime | |

Indexes: `{ conversation_id: 1, created_at: 1 }` (one thread, oldest-first);
`{ group_id: 1, ticket_id: 1 }` (cascades);
`{ role: 1, user_id: 1, created_at: 1 }` (the rate-limit count — equality on role
and user, range on time).

The rate-limit index is global per user rather than per conversation, matching the
rule it serves: 10 user messages per hour per person, across all tickets.

### `admin_audit_log`

System Admin actions worth a permanent trail. Not tenant data: written by System
Admin, never read by group-scoped business logic.

| Field | Type | Notes |
| --- | --- | --- |
| `_id` | ObjectId | |
| `action` | `succession` \| `group_auto_deleted` \| `promotion` \| `demotion` | |
| `group_id` | ObjectId \| null | Succession / auto-delete only. |
| `group_name` | String | Snapshotted. |
| `deleted_user_id`, `deleted_user_name` | ObjectId \| null, String | The user being deleted. |
| `successor_user_id`, `successor_user_name` | ObjectId \| null, String \| null | Succession only. |
| `target_user_id`, `target_user_name` | ObjectId \| null, String \| null | Promotion/demotion only. |
| `performed_by`, `performed_by_name` | ObjectId, String | The acting admin. |
| `created_at` | DateTime | |

Indexes: `{ group_id: 1 }` and `{ deleted_user_id: 1 }`, serving the viewer's two
independent filters.

Most fields are optional because they are action-specific; only `performed_by`
and `action` are universal. **Names are snapshotted at write time** — the whole
point of the log is to survive the deletion of the things it references, so ids
often cannot be resolved by the time anyone reads it. Every name field carries
`#[serde(default)]` so entries written before a field existed still deserialize
rather than failing the entire query.

---

## Relationships

```
users ──< group_members >── groups
                              │
                              └──< tickets ──< comments ──< comment_reactions
                                     ├──< ticket_activity
                                     ├──< ticket_links (both ends, same group)
                                     ├──< ticket_references
                                     ├──── ai_ticket_insights  (1:1)
                                     └──< ai_conversations ──< ai_chat_messages

users ──< refresh_tokens
admin_audit_log — references users/groups by snapshotted id + name, no live FK
```

## Cascades

Nothing cascades in the database; services do it explicitly.

**Deleting a ticket** clears its comments, comment reactions, activity, links
(matching on either end), references, AI insights, conversations, and chat
messages.

**Deleting a group** runs `group::purge_group_data`, which does the above across
every ticket in the group, then removes the tickets, the membership rows, and the
group itself. The group's `counters` document goes with it — that happens inside
`TicketRepository::delete_by_group`, alongside the tickets, rather than as a
separate step in `purge_group_data`. It is also what runs when a group is
auto-deleted because its only member was deleted.

**Deleting a user** does *not* delete their content. Tickets, comments, and
activity keep the original `created_by` / `user_id` / `actor_id`, and the name
lookup falls back to an empty string, so history stays intact rather than
silently reattributing or vanishing.

## Uniqueness and races

Six unique indexes double as concurrency guards: `users.email`,
`refresh_tokens.token_hash`, `group_members(group_id, user_id)`,
`tickets(group_id, ticket_number)`, `ticket_links(…)`, and
`comment_reactions(comment_id, user_id)`. The application-level pre-checks exist
to produce good error messages; the index is what actually rejects a concurrent
duplicate. `utils::is_duplicate_key` recognizes error 11000 in both shapes it
arrives in — a `WriteError` on plain inserts, a `CommandError` on
`find_one_and_update`.

## Index management and migrations

`db::ensure_indexes` runs on every boot from a single table in `db.rs`, so
indexes are code, not an operational step. Adding one means adding a row.

Two one-time migrations also run at every boot and are written to be idempotent:

- `backfill_ticket_content_updated_at` — copies `updated_at` into
  `content_updated_at` for tickets predating that field. Required rather than
  optional: without it those documents fail to deserialize and 500 any read that
  touches them. It uses a pipeline update, since a plain `$set` cannot copy
  another field's value.
- `wipe_legacy_chat_messages` — drops chat messages from the old
  single-shared-thread-per-ticket model (no `conversation_id`, several users'
  messages interleaved with no owner). There was no principled way to split those
  into per-user conversations after the fact, so they are removed rather than
  migrated.

Both filter on the condition they fix, so they are no-ops after the first run.
