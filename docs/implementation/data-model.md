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

Seven, of which **five are written by application code**:

| Collection | Rust type | Written by | Purpose |
|---|---|---|---|
| `users` | `user::models::User` | `UserRepository` | Accounts + global role |
| `refresh_tokens` | `auth::models::RefreshTokenDoc` | `AuthRepository` | Session records |
| `groups` | `group::models::Group` | `GroupRepository` | Tenant boundary |
| `group_members` | `group::models::GroupMember` | `GroupRepository` | Membership + group role (the RBAC table) |
| `tickets` | `ticket::models::Ticket` | `TicketRepository` | Core business entity |
| `counters` | `ticket::models::TicketCounter` | `TicketRepository` | Per-group ticket-number sequence |
| `admin_audit_log` | `admin::models::AuditLogEntry` | `AdminRepository` | Succession / auto-deletion trail |

**Do not exist in code:** `comments`, `ai_ticket_insights`, `ai_group_reports`. They are
specified in `docs/specification/database.md`, but the `comment/` and `ai/` Rust modules are
empty files — nothing creates, reads, or indexes them.

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

**Not implemented** (would exist if comments/AI were built): `tickets → comments` (1-to-many),
`tickets → ai_ticket_insights`, `groups → ai_group_reports`.

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
| Group deleted | Memberships are removed; **its tickets and its `counters` row are left orphaned.** See [`deviations.md`](./deviations.md). |
| Membership added for a nonexistent `user_id` | Possible via a hand-crafted `POST /groups/{id}/users`; `enrich_member` renders empty name/email. |
| Audit log references deleted entities | Handled by design — names are snapshotted at write time (`group_name`, `deleted_user_name`, ...) precisely because the ids won't resolve later. |

The audit log is the only place the schema deliberately denormalizes to survive deletion.
Everywhere else, joins are done at read time by a second query (`enrich_member`,
`enrich_ticket`) and tolerate a missing target by substituting an empty string.

---

## Isolation model

Two tiers of data:

**Tenant data — must always be queried with `group_id`:**
`tickets`, `group_members`, `counters` (whose `_id` *is* the group id).

Every query in `TicketRepository` includes `group_id` in its filter document, including
single-document reads: `find_by_id(group_id, ticket_id)` filters on both `_id` and
`group_id`. That's what makes a ticket id from another group unresolvable rather than
merely unauthorized.

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

Where atomicity actually matters, it's pushed into single-document operations:

- **`counters`** — `find_one_and_update` + `$inc` + `upsert` allocates a ticket number atomically. No check-then-insert race.
- **Unique indexes** do the work that check-then-insert would otherwise do racily: `users.email` (duplicate registration), `group_members (group_id, user_id)` (duplicate membership), `tickets (group_id, ticket_number)` (sequence collision). In each case the repository inserts optimistically and maps error code `11000` to a domain error → `409`.
- **`refresh_tokens.expires_at`** carries a TTL index (`expireAfterSeconds: 0`), so Mongo's background reaper deletes spent and expired sessions with no cleanup job.

Single-use refresh tokens are enforced by *query shape* rather than a lock:
`find_active_by_hash` filters `{token_hash, revoked_at: null, expires_at: {$gt: now}}`, so a
replayed token simply isn't found.
