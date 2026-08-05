# Indexes

All indexes are created by `db::ensure_indexes()` in `backend/src/db.rs`, called once at
startup from `main()`. `createIndex` is idempotent in MongoDB, so this runs safely on every
boot and there is no migration step.

**Sixteen indexes across eight collections** (plus the implicit `_id` index Mongo creates for
every collection).

---

## Full list

| # | Collection | Keys | Options | Serves |
|---|---|---|---|---|
| 1 | `users` | `email: 1` | **unique** | `find_by_email` (login, member lookup); enforces one account per email |
| 2 | `refresh_tokens` | `token_hash: 1` | **unique** | `find_active_by_hash`, `revoke_by_hash` |
| 3 | `refresh_tokens` | `expires_at: 1` | **TTL**, `expireAfterSeconds: 0` | Automatic cleanup — not query performance |
| 4 | `group_members` | `group_id: 1, user_id: 1` | **unique** | `find_member` (every RBAC check); enforces one membership per pair |
| 5 | `group_members` | `user_id: 1` | — | `list_memberships_for_user`, `list_groups_for_user` |
| 6 | `admin_audit_log` | `group_id: 1` | — | `GET /admin/audit-log?group_id=` |
| 7 | `admin_audit_log` | `deleted_user_id: 1` | — | `GET /admin/audit-log?user_id=` |
| 8 | `tickets` | `group_id: 1` | — | Every ticket query (isolation + base scan) |
| 9 | `tickets` | `group_id: 1, status: 1` | — | Status filter; `count_open_by_group` |
| 10 | `tickets` | `group_id: 1, created_by: 1` | — | `creator` filter |
| 11 | `tickets` | `group_id: 1, ticket_number: 1` | **unique** | Guards the per-group sequence |
| 12 | `comments` | `group_id: 1, ticket_id: 1` | — | `list_by_ticket`; `delete_by_ticket` (both keys); `delete_by_group` (`group_id` prefix) |
| 13 | `comments` | `parent_comment_id: 1` | — | `has_replies` — the hard-vs-soft-delete check |
| 14 | `ai_ticket_insights` | `group_id: 1, ticket_id: 1` | **unique** | `find_insight`; guards the "one document per ticket" invariant |
| 15 | `ai_group_reports` | `group_id: 1, generated_at: -1` | — | `find_latest_report` — the newest report for a group |
| 16 | `ai_group_reports` | `generated_at: 1` | **TTL**, `expireAfterSeconds: 2592000` (30 days) | Automatic cleanup of old reports — not query performance |

---

## Why each one exists

### Unique indexes doing double duty

Five of the sixteen are `unique`. In three cases uniqueness isn't just a constraint —
it's the **concurrency-control mechanism**. Rather than "check whether it exists, then
insert" (racy), the repository inserts optimistically and translates Mongo error code
`11000` into a domain error:

| Index | Race it closes | Error mapping |
|---|---|---|
| `users.email` | Two simultaneous registrations with the same email | `UserRepoError::DuplicateEmail` → `409 duplicate_email` |
| `group_members (group_id, user_id)` | Two simultaneous adds of the same member | `GroupRepoError::DuplicateMember` → `409 conflict` |
| `tickets (group_id, ticket_number)` | Sequence collision | (defense only — the atomic counter already prevents this) |

The fifth, `ai_ticket_insights (group_id, ticket_id)`, is a different case: it's not backing a
check-then-insert race at all. Every write to this collection goes through
`find_one_and_update(..., upsert: true)` (`AiRepository::upsert_summary`/`upsert_analysis`),
which is atomic on its own — two concurrent calls for the same ticket can't create two
documents regardless of the index. The unique index here is a schema guarantee ("one insight
per ticket," matching `08-ai.md`'s "upserted in place" design) and a backstop, not a
race-closer, so there's no corresponding duplicate-key error mapping in `AiRepoError`.

`is_duplicate_key` in `user/repository.rs` checks **two** error shapes —
`ErrorKind::Write(WriteError)` for `insert_one` and `ErrorKind::Command` for
`find_one_and_update` (because `findAndModify` is a command, not a plain write). The group
repository's version only checks the write shape, which is sufficient there since
`insert_member` is its only unique-index write.

### The two `group_members` indexes

They are not redundant. The compound index `(group_id, user_id)` cannot serve a query that
filters on `user_id` alone, because **`user_id` isn't its prefix** — Mongo can only use a
compound index for a query matching a leading subset of its keys. Since "list my groups"
(`list_memberships_for_user`) filters on `user_id` only, it needs its own index.

The compound index is ordered `(group_id, user_id)` rather than the reverse because
`find_member` — the hottest query in the system, running on every group-scoped request —
supplies both, and `group_id` is the more selective leading field for any partial use.

### The TTL index

Index #3 is not for lookups. `expireAfterSeconds: 0` means "delete this document once
`expires_at` is in the past", and Mongo's background reaper (running roughly once a minute)
does it. That's why nothing in the codebase cleans up spent or expired refresh tokens — the
collection self-limits to approximately the number of live sessions.

Note it only reaps by `expires_at`. A token revoked at logout on day 1 still sits in the
collection until its original 30-day expiry, harmless because `find_active_by_hash` filters
on `revoked_at: null`.

### The `ai_group_reports` indexes (#15, #16)

Two separate single-field-or-compound indexes on the same collection, serving different
concerns — same reasoning as the two `group_members` indexes above (a compound index can't
serve a query that isn't a prefix match of its keys, and **MongoDB TTL indexes specifically
cannot be compound at all**, so #16 couldn't be folded into #15 even if the query patterns
allowed it).

- **#15** `(group_id, generated_at: -1)` serves `find_latest_report`'s "most recent report for
  this group" query.
- **#16** `(generated_at: 1)`, TTL, is unrelated to lookups — same mechanism as index #3
  (`refresh_tokens.expires_at`), but *relative* rather than absolute: `expireAfterSeconds:
  2592000` means "delete 30 days after the value in `generated_at`," vs. refresh_tokens'
  `expireAfterSeconds: 0` meaning "delete once `expires_at` itself is in the past." Exists
  because `ai_group_reports` is insert-only — one new document per generation, not upserted
  in place like `ai_ticket_insights` — so an actively-used group regenerating its report on
  every TTL expiry (`08-ai.md`: one hour) would otherwise accumulate unbounded history that
  nothing ever reads (`find_latest_report` only ever wants the newest one).

### The ticket index family

All four lead with `group_id`, which mirrors the isolation rule exactly: every ticket query
filters on it, so it belongs first in every index.

- **#8** covers the plain "all tickets in this group" case and the base of every filtered query.
- **#9** covers `?status=` and `count_open_by_group` (which `GroupService::list_my_groups` calls once per group).
- **#10** covers `?creator=`.
- **#11** is uniqueness insurance layered on the atomic counter.

**Not covered by any index:** free-text title search (`?q=`). There is no text index and no
`$regex` on title — search runs **in-process** in `TicketService::search_by_title`, over the
result set the indexed filters already produced. Combined filters (e.g. status **and**
priority) also use only one index; Mongo picks the most selective and filters the rest in
memory.

### The two audit-log indexes

Deliberately **separate single-field indexes rather than one compound**, because the two
filters are independent — either may be used alone, both may be combined, both may be
omitted. A compound `(group_id, deleted_user_id)` couldn't serve a `deleted_user_id`-only query.

There is **no index on `created_at`**, even though `list_audit_log` always sorts by it
descending. Mongo therefore does an in-memory sort. Fine for a low-volume system-metadata
collection; it would become the first thing to fix at scale (Mongo aborts in-memory sorts
above 32 MB).

---

## Collections with no secondary indexes

- **`groups`** — nothing queries it by anything but `_id`. `owner_id` is informational and never filtered on. The admin group list (`list_all_groups`) is an unindexed collection scan with an optional name regex; acceptable because the group count is small and the endpoint is admin-only.
- **`counters`** — every access is by `_id` (which *is* the `group_id`), served by the implicit `_id` index.

---

## Query patterns without index support

Honest list, for when you're asked about scaling:

| Query | Why it's unindexed | Impact |
|---|---|---|
| `UserRepository::list_all` with `search` | `$or` over two `$regex` fields; a non-anchored, case-insensitive regex can't use an index | Full collection scan. Admin-only. |
| `GroupRepository::list_all_groups` with `search` | Same | Full scan. Admin-only. |
| `TicketService::search_by_title` (`?q=`) | Runs in application memory, not Mongo | Whole group-filtered set loaded before searching |
| Ticket pagination | `.skip()/.take()` applied in Rust, not Mongo | Whole filtered set loaded per page request |
| `admin_audit_log` sort by `created_at` | No index on the sort key | In-memory sort |
| `GroupService::list_my_groups` | N+1: 1 query for memberships, then 3 per group | Sequential round trips, not a missing index |

None of these are bugs at the current scale, and each has a clear fix (a text index, pushing
`skip`/`limit` into the query, a `created_at` index, `$lookup` or concurrent futures).

---

## Where the code diverges from the spec

The spec lists `comments` (`ticket_id`, `group_id`) as two separate single-key indexes.
The code creates **one compound** `(group_id, ticket_id)` instead, which serves both the
per-ticket read and the per-group cascade (via its prefix) with one index — plus a
`parent_comment_id` index the spec doesn't mention, since the spec's `comments` shape has no
threading field.

`docs/specification/database.md` lists indexes for `ai_ticket_insights` on `ticket_id` and
`group_id` as two separate single-key indexes. The code creates **one unique compound**
`(group_id, ticket_id)` instead (index #14 above) — same "one compound covers both the lookup
and the isolation filter" reasoning as `comments`, plus `unique` to enforce the one-document-
per-ticket invariant the spec's plain single-key indexes wouldn't. The spec says nothing about
`ai_group_reports`' indexes or about a TTL policy for either AI collection; #15 and #16 (and
the 30-day retention window) are both implementation decisions made when the AI feature was
built — see `08-ai.md`.
