# Data Model

Derived from the Rust structs and Mongo queries in `backend/src/`, not from
`docs/specification/database.md`. Where the two disagree, see [`deviations.md`](./deviations.md).

Detail files:
- [`db/collections.md`](./db/collections.md) — field-by-field, per collection
- [`db/indexes.md`](./db/indexes.md) — every index, and which query it serves

---

## Database

MongoDB. Single database, name **hardcoded** as `"resolve"` in `db::database()`. Connection
string comes from `MONGO_URI` (the `.env.example` shows a `mongodb+srv://` Atlas cluster).

There is no ORM/ODM and no migration system. Schema lives entirely in the Rust structs, and
`serde` is what enforces it on read: a document that doesn't deserialize into its struct
fails the query. Indexes are created at boot by `db::ensure_indexes()`.

---

## Collections that actually exist

Eleven, of which **nine are written by application code**:

| Collection | Rust type | Written by | Purpose |
|---|---|---|---|
| `users` | `user::models::User` | `UserRepository` | Accounts + global role |
| `refresh_tokens` | `auth::models::RefreshTokenDoc` | `AuthRepository` | Session records |
| `groups` | `group::models::Group` | `GroupRepository` | Tenant boundary |
| `group_members` | `group::models::GroupMember` | `GroupRepository` | Membership + group role (the RBAC table) |
| `tickets` | `ticket::models::Ticket` | `TicketRepository` | Core business entity |
| `counters` | `ticket::models::TicketCounter` | `TicketRepository` | Per-group ticket-number sequence |
| `comments` | `comment::models::Comment` | `CommentRepository` | Threaded ticket discussion |
| `admin_audit_log` | `admin::models::AuditLogEntry` | `AdminRepository` | Succession / auto-deletion trail |
| `ai_ticket_insights` | `ai::models::AiTicketInsight` | `AiRepository` | Cached per-ticket summary/analysis (`08-ai.md`) |
| `ai_group_reports` | `ai::models::AiGroupReport` | `AiRepository` | Cached group analytics reports, TTL-expired after 30 days |
| `ai_chat_messages` | `ai::models::ChatMessage` | `AiRepository` | Per-ticket chat thread with the AI, one document per message (`08-ai.md`) |

---

## Entity-relationship overview

```
                    ┌─────────────────┐
                    │      users      │
                    │  _id            │
                    │  email (unique) │
                    │  password_hash  │
                    │  global_role?   │
                    └────────┬────────┘
                             │
        ┌────────────────────┼─────────────────────┬──────────────────┐
        │ 1                  │ 1                   │ 1                │ 1
        │                    │                     │                  │
        ▼ N                  ▼ N                   ▼ N                ▼ N
┌───────────────┐   ┌─────────────────┐   ┌──────────────┐   ┌─────────────────┐
│refresh_tokens │   │  group_members  │   │   tickets    │   │ admin_audit_log │
│  user_id ─────┘   │  user_id ───────┘   │  created_by ─┘   │ deleted_user_id ┘
│  token_hash   │   │  group_id ──┐   │   │  group_id ──┐│   │ performed_by    │
│  expires_at   │   │  role       │   │   │             ││   │ successor_user_id│
└───────────────┘   └─────────────┼───┘   └─────────────┼┘   │ group_id        │
                                  │                     │    └────────┬────────┘
                                  │  N                  │ N           │ N
                                  ▼                     ▼             ▼
                          ┌──────────────────────────────────────────────┐
                          │                   groups                     │
                          │  _id, name, owner_id, created_at             │
                          └──────────────────┬───────────────────────────┘
                                             │ 1
                                             ▼ 1
                                      ┌──────────────┐
                                      │   counters   │
                                      │ _id==group_id│
                                      │ ticket_seq   │
                                      └──────────────┘

           tickets ──1──▶ N──┐
                             ▼            ┌──── self-reference (any depth)
                    ┌──────────────────┐  │
                    │     comments     │◀─┘
                    │  _id             │
                    │  group_id  ──────┼──▶ groups   (denormalized, not via ticket)
                    │  ticket_id ──────┼──▶ tickets
                    │  parent_comment_id?  (null = top-level)
                    │  user_id   ──────┼──▶ users
                    │  content         │
                    │  is_deleted      │
                    └──────────────────┘
```

### Relationship table

| From | To | Cardinality | Implemented as |
|---|---|---|---|
| `users` | `refresh_tokens` | **1-to-many** | `refresh_tokens.user_id` |
| `users` ↔ `groups` | — | **many-to-many** | join collection `group_members`, which carries `role` as join-row data |
| `groups` | `group_members` | **1-to-many** | `group_members.group_id` |
| `users` | `group_members` | **1-to-many** | `group_members.user_id` |
| `groups` | `tickets` | **1-to-many** | `tickets.group_id` |
| `users` | `tickets` | **1-to-many** (as creator) | `tickets.created_by` |
| `groups` | `counters` | **1-to-1** | `counters._id == group_id` |
| `groups` | `admin_audit_log` | **1-to-many** | `admin_audit_log.group_id` |
| `users` | `admin_audit_log` | **1-to-many**, three separate ways | `deleted_user_id`, `performed_by`, `successor_user_id` |
| `tickets` | `comments` | **1-to-many** | `comments.ticket_id` |
| `groups` | `comments` | **1-to-many** | `comments.group_id` — denormalized, *not* resolved through the ticket |
| `users` | `comments` | **1-to-many** (as author) | `comments.user_id` |
| `comments` | `comments` | **1-to-many, self-referential** | `comments.parent_comment_id`, nullable, **no depth limit** |
| `tickets` | `ai_ticket_insights` | **1-to-1** | `ai_ticket_insights.ticket_id` — one document, upserted in place per generation, not a fresh row each time |
| `groups` | `ai_ticket_insights` | **1-to-many** | `ai_ticket_insights.group_id` — denormalized, same reasoning as `comments.group_id` below |
| `groups` | `ai_group_reports` | **1-to-many** | `ai_group_reports.group_id` — a fresh document per generation (history), TTL-expired after 30 days |
| `users` | `ai_group_reports` | **1-to-many** (as generator) | `ai_group_reports.generated_by` |
| `tickets` | `ai_chat_messages` | **1-to-many** | `ai_chat_messages.ticket_id` — one ongoing conversation's worth of messages, no separate thread id |
| `groups` | `ai_chat_messages` | **1-to-many** | `ai_chat_messages.group_id` — denormalized, same reasoning as `comments.group_id` |
| `users` | `ai_chat_messages` | **1-to-many** (as author) | `ai_chat_messages.user_id` — `null` on an assistant message, so this relationship only holds for the user-authored half of the thread |

The entity-relationship diagram above predates the AI module and doesn't show these seven —
see [`backend/08-ai.md`](./backend/08-ai.md) for the full field-by-field breakdown.

`comments` is the schema's only self-reference, and the only place a child row duplicates its
grandparent's id (`group_id`) rather than joining up through its parent. That duplication is
deliberate: it keeps every comment query group-filterable on its own, and makes the
group-deletion cascade a single `delete_many({group_id})` instead of a fan-out over tickets.

---

## The one many-to-many, and why it carries data

`group_members` is not a bare join table — it's the RBAC store. Its `role` field is an
attribute *of the relationship*, not of either entity:

- The same user is `group_admin` in one group and `contributor` in another. Role is meaningless without both ids.
- Every authorization decision reads this row (`GroupRepository::find_member`), never `users` and never `groups.owner_id`.

Consequences worth stating:
- A user with zero `group_members` rows can log in and see nothing but their account page.
- `groups.owner_id` exists but is **decorative** — deleting the owner's membership does not transfer or revoke anything, and no query filters on it.
- The invariant "every group has ≥1 `group_admin` row" is enforced only in application code (`GroupService::guard_sole_admin_removal`), never by the database.

---

## Referential integrity: there is none

MongoDB enforces no foreign keys, and this codebase adds no application-level equivalent
except where noted. What that means in practice:

| Situation | Actual behavior |
|---|---|
| User deleted while they created tickets | Tickets keep the dangling `created_by`. `TicketService::enrich_ticket` renders `created_by_name` as `""`. |
| User deleted while a member of groups | Handled — `AdminService::delete_user` removes every membership first. |
| User deleted while they authored comments | Comments keep the dangling `user_id`. `CommentService::enrich_comment` renders `user_name` as `""`. |
| Group deleted | Handled — `purge_group_data` cascades memberships, tickets, the `counters` row, and comments, deleting the group document last. |
| Ticket deleted | Handled — `TicketService::delete_ticket` cascades the ticket's comments first, then the ticket. |
| Comment deleted while it has replies | Handled by **tombstoning** instead of deleting: the row survives with `is_deleted: true` so its replies' `parent_comment_id` still resolves. A leaf comment is hard-deleted. |
| Membership added for a nonexistent `user_id` | Possible via a hand-crafted `POST /groups/{id}/users`; `enrich_member` renders empty name/email. |
| Audit log references deleted entities | Handled by design — names are snapshotted at write time (`group_name`, `deleted_user_name`, ...) precisely because the ids won't resolve later. |

The audit log is the only place the schema deliberately denormalizes to survive deletion.
Everywhere else, joins are done at read time by a second query (`enrich_member`,
`enrich_ticket`) and tolerate a missing target by substituting an empty string.

---

## Isolation model

Two tiers of data:

**Tenant data — must always be queried with `group_id`:**
`tickets`, `comments`, `group_members`, `counters` (whose `_id` *is* the group id).

Every query in `TicketRepository` includes `group_id` in its filter document, including
single-document reads: `find_by_id(group_id, ticket_id)` filters on both `_id` and
`group_id`. That's what makes a ticket id from another group unresolvable rather than
merely unauthorized. `CommentRepository` does the same one level deeper —
`find_by_id(group_id, ticket_id, comment_id)` filters on all three.

**One documented exception:** `CommentRepository::has_replies` filters on
`parent_comment_id` alone. It's only ever called on a comment already fetched through a
group-filtered `find_by_id`, so the boundary has been established before it runs.

The comment module also shows the limit of the filter-shape mechanism. Filtering
`comments.group_id` proves nothing when `group_id` is the *caller's own* — it's the
`ticket_id` in the path that could belong elsewhere. `CommentService::require_ticket_in_group`
is the explicit check that covers it, and it exists because a live test found the gap.

**Non-tenant data — legitimately queried without `group_id`:**
`users` (by `_id`/`email`, or listed system-wide by admin), `refresh_tokens` (by
`token_hash`/`user_id`), `groups` (by `_id`, or listed system-wide by admin),
`admin_audit_log` (system metadata).

This distinction matters: `docs/specification/backend.md` states "EVERY database query MUST
include group_id filter. No exceptions." Taken literally that's false of the working code —
the real rule is the tenant/non-tenant split above.

---

## ID and type conventions

- **`_id`** is always a Mongo `ObjectId`. In Rust it's `Option<ObjectId>` with `#[serde(rename = "_id", skip_serializing_if = "Option::is_none")]`, so `None` on insert lets Mongo generate it. The repository then returns the struct with the id filled in from `inserted_id`, avoiding a read-after-write.
- **`counters` is the exception** — its `_id` is the group's `ObjectId`, non-optional, giving a natural 1-to-1 with `groups` and free uniqueness.
- **Timestamps are stored as `mongodb::bson::DateTime`** (BSON date, millisecond precision) and converted to `chrono::DateTime<Utc>` in every `*Response` type, which serializes to RFC3339 for the API. Conversions use `.unwrap_or_default()`, so an out-of-range value yields the epoch rather than a panic.
- **Enums are stored as strings**, using each type's serde representation. `Role`, `TicketStatus`, `TicketPriority`, and `AuditAction` all use `rename_all = "snake_case"`. **`GlobalRole` does not** — it stores `"SystemAdmin"`. Queries that filter on an enum use `bson::to_bson(&value)` rather than a literal, so the rename stays the single source of truth (the one exception is `count_open_by_group`, which matches the literal `"open"`).
- **IDs cross the API as hex strings**, parsed back to `ObjectId` at the handler boundary via `parse_id`, which maps a bad id to `400 validation_error`.

---

## Atomicity and consistency

**No Mongo transactions are used anywhere.** Two multi-write flows accept that explicitly:

1. `GroupService::create_group` — insert group, then insert the creator's membership. A failure between them leaves a group with no members.
2. `AdminService::delete_user` — many writes across groups, ordered so the user document is deleted **last**, making a retry after partial failure safe and convergent.
3. The two cascades — `purge_group_data` and `TicketService::delete_ticket` — use the same ordering trick: children first, the parent document last, so a mid-failure leaves the parent still resolvable and the cascade re-runnable.

Where atomicity actually matters, it's pushed into single-document operations:

- **`counters`** — `find_one_and_update` + `$inc` + `upsert` allocates a ticket number atomically. No check-then-insert race.
- **Unique indexes** do the work that check-then-insert would otherwise do racily: `users.email` (duplicate registration), `group_members (group_id, user_id)` (duplicate membership), `tickets (group_id, ticket_number)` (sequence collision). In each case the repository inserts optimistically and maps error code `11000` to a domain error → `409`.
- **`refresh_tokens.expires_at`** carries a TTL index (`expireAfterSeconds: 0`), so Mongo's background reaper deletes spent and expired sessions with no cleanup job.

Single-use refresh tokens are enforced by *query shape* rather than a lock:
`find_active_by_hash` filters `{token_hash, revoked_at: null, expires_at: {$gt: now}}`, so a
replayed token simply isn't found.
