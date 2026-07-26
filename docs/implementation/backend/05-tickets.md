# Backend — Tickets

Covers `src/ticket/`: `models.rs` (152), `repository.rs` (173), `service.rs` (280),
`handlers.rs` (140).

Tickets are the core business entity and the module with the most interesting query logic:
an atomic per-group sequence, a hybrid Mongo/in-process search, and in-memory pagination.

Routes live under the group scope (`/groups/{id}/tickets/...`), registered in
`server/routes.rs`, so `GroupScoped` can read `{id}`.

---

## `ticket/models.rs`

### `enum TicketStatus { Open, Closed }`
`snake_case` → `"open"` / `"closed"`. Only two states — no `in_progress`.

### `enum TicketPriority { Low, High, Critical }`
`snake_case` → `"low"` / `"high"` / `"critical"`. Note there is **no `medium`**, which is
the value people assume exists; the inline tests explicitly assert it's rejected.

Both enums derive `Copy, PartialEq, Eq` and are validated by serde at deserialization —
an unknown status/priority in a request body or query string fails to deserialize and
Actix returns `400` before any handler code runs. That's the whole validation story for
these two fields.

### `struct Ticket`
Stored document: `id`, `group_id`, `ticket_number: i64`, `title`, `description`, `status`,
`priority`, `created_by: ObjectId`, `created_at`, `updated_at`.

- `ticket_number` is the **human-facing** identifier — a running number scoped to the group (1, 2, 3… independently per group), distinct from `_id`.
- **No `assignee` field.** Tickets are never assigned to anyone; that's a deliberate product decision, not an omission.

### `struct CreateTicketInput { group_id, ticket_number, title, description, priority, created_by }`
Repository input. Note `status` is absent — the repository always sets `Open`.

### `struct TicketCounter { group_id (as `_id`), ticket_seq: i64 }`
One document per group in the `counters` collection, keyed by `group_id` as `_id`. Backs the
per-group sequence.

### `struct TicketResponse`
Client shape: all ticket fields with ObjectIds as hex strings and timestamps as
`DateTime<Utc>`, **plus `created_by_name: String`** — a denormalized join, filled by
`enrich_ticket`.

### `struct CreateTicketRequest { title, description, priority }`
Body of `POST`. `status`, `ticket_number`, and `created_by` are server-assigned and
**cannot be supplied by the client** — they're simply not fields on this struct.

### `struct UpdateTicketRequest { title: Option, description: Option, priority: Option, status: Option }`
Body of `PATCH`. All optional; the handler rejects an all-absent body.

### `struct ListTicketsQuery { q, status, priority, creator, page, per_page }`
Deserialized straight from the query string via `web::Query`, so every field is `Option`.
`creator` stays a raw `Option<String>` and is parsed to `ObjectId` in the service (matching
how other id params are handled). `status`/`priority` are typed, so an invalid value is a
`400` from serde.

### `struct TicketListResponse { items: Vec<TicketResponse>, total: u64, page: u64, per_page: u64 }`
The paginated envelope. `total` is the count **after** all filters and search, not the
group's total ticket count.

### Inline tests
Four, all on serialization: status and priority serialize snake_case, and both reject
unknown values (`"in_progress"`, `"medium"`).

---

## `ticket/repository.rs`

### `enum TicketRepoError { Database(_) }`
Single variant — there's no domain-specific failure to distinguish here (no unique
constraint the app expects to trip). Maps to `ApiError::Internal`.

### `struct TicketRepository { tickets: Collection<Ticket>, counters: Collection<TicketCounter> }`
Two collections: `"tickets"` and `"counters"`.

### `async fn next_ticket_number(&self, group_id) -> Result<i64, TicketRepoError>`
**The atomic sequence allocator.**

```rust
counters.find_one_and_update(
    doc! { "_id": group_id },
    doc! { "$inc": { "ticket_seq": 1i64 } },
).upsert(true).return_document(ReturnDocument::After)
```

`upsert(true)` creates the counter on the group's first ticket (starting at 1);
`ReturnDocument::After` returns the post-increment value. Because `findAndModify` is a
single atomic server-side operation, two tickets created in the same group at the same
instant get distinct numbers with no check-then-insert race. `.expect("upsert always
returns a document")` is safe given `After` + `upsert`.

The unique index on `(group_id, ticket_number)` is a second line of defense on top of this.

### `async fn insert_ticket(&self, input) -> Result<Ticket, TicketRepoError>`
Sets `status: Open` and `created_at == updated_at == now`, inserts, returns the struct with
the generated `_id`.

### `async fn find_by_id(&self, group_id, ticket_id) -> Result<Option<Ticket>, _>`
**Filters on `_id` AND `group_id`.** This is the group-isolation mechanism in its clearest
form: a valid ticket id paired with a different group's id matches nothing → `None` → `404`.
Not an authorization check — a query shape.

### `async fn list_by_group(&self, group_id, status, priority, creator) -> Result<Vec<Ticket>, _>`
Builds `doc! { "group_id": group_id }` and conditionally inserts `status`, `priority`
(via `bson::to_bson` on the enum, so the serde rename stays authoritative), and
`created_by`. Collects the entire cursor.

The comment explains the division of labor: these three are exact-match, indexable fields so
they're filtered in Mongo; free-text title search has no Mongo-native equivalent here and is
done in-process by the service over this result.

**No `.limit()` / `.skip()`** — pagination happens later, in memory.

### `async fn update_ticket(&self, group_id, ticket_id, changes: Document) -> Result<Option<Ticket>, _>`
`find_one_and_update` filtered on `{_id, group_id}` with `$set: changes`, returning the
updated document. Same isolation guarantee as `find_by_id`. Takes a pre-built `Document`,
so the service decides which fields are in play.

### `async fn delete_ticket(&self, group_id, ticket_id) -> Result<bool, _>`
`delete_one` on `{_id, group_id}`.

### `async fn count_open_by_group(&self, group_id) -> Result<u64, _>`
`count_documents({group_id, status: "open"})`. Matches the **string literal** rather than
round-tripping the enum — the comment notes status is stored as its snake_case
serialization. Served by the `(group_id, status)` index. This is the method `GroupService`
reaches across for.

---

## `ticket/service.rs`

### Constants
- `DEFAULT_PER_PAGE: u64 = 20`
- `MAX_PER_PAGE: u64 = 100`

### `struct TicketService { repo, user_service, rbac }`

### `async fn create_ticket(&self, user_id, group_id, input) -> Result<TicketResponse, ApiError>`
`require_member` (any member may create) → `next_ticket_number` → `insert_ticket` with
`created_by: user_id` → `enrich_ticket`.

### `async fn get_ticket(&self, user_id, group_id, ticket_id)`
`require_member` → `find_by_id(group_id, ticket_id)` → `ok_or(NotFound)` → `enrich_ticket`.

### `async fn list_tickets(&self, user_id, group_id, query) -> Result<TicketListResponse, ApiError>`
The most involved method in the backend. Order of operations matters:

1. `require_member`.
2. Parse `query.creator` → `ObjectId`; a malformed value → `Validation("invalid creator id")`.
3. `repo.list_by_group(group_id, status, priority, creator)` — **the group filter is applied here, in the database, before anything else touches the data.**
4. If `q` is present and non-blank (after `trim`), `tickets = search_by_title(tickets, term)`.
5. `total = tickets.len()` — post-filter, post-search.
6. `page = query.page.unwrap_or(1).max(1)` (page 0 becomes 1); `per_page = query.per_page.unwrap_or(20).clamp(1, 100)`.
7. `start = (page - 1) * per_page`; then `.into_iter().skip(start).take(per_page)`.
8. `enrich_ticket` **per item on the current page only** — so the extra user lookups scale with page size, not with the result set.

The cost model to be honest about: steps 3–7 pull the entire group-filtered set into memory
before paging. It's correct and simple, and it's the one place the backend doesn't push work
down to the database.

### `async fn update_ticket(&self, user_id, group_id, ticket_id, input)`
`require_group_admin` — **Group Admin only, including status changes, and including the
ticket's own creator.** A Contributor can open a ticket and then cannot touch it again.

Builds a `Document` from whichever fields are `Some`, enum fields via `bson::to_bson`, then
**always** inserts `updated_at: now`. Note the consequence: because `updated_at` is always
set, the `$set` document is never empty — but the handler has already rejected an all-absent
body, so that path isn't reachable.

`update_ticket` → `ok_or(NotFound)` → `enrich_ticket`.

### `async fn delete_ticket(&self, user_id, group_id, ticket_id) -> Result<(), ApiError>`
`require_group_admin` → `delete_ticket` → `if !deleted { NotFound }`.

### `async fn enrich_ticket(&self, ticket: Ticket) -> Result<TicketResponse, ApiError>` (private)
One `user_service.find_by_id(ticket.created_by)` to attach `created_by_name`. A deleted
creator yields `""` rather than an error. The comment notes this mirrors
`GroupService::enrich_member` and makes the same one-lookup-per-row-over-`$lookup` tradeoff.

### `fn search_by_title(tickets: Vec<Ticket>, term: &str) -> Vec<Ticket>` (free function, private)

Two-stage, and the second stage only runs if the first finds nothing:

**Stage 1 — substring.** Lowercase both sides, keep tickets whose title contains the needle.
If any match, return them immediately.

**Stage 2 — typo-tolerant fallback.**
- `max_distance = (needle.chars().count() / 3).max(1)` — roughly one edit per three characters, minimum 1. Short queries therefore still demand a near-exact match.
- For each ticket, split the lowercased title on whitespace and take the **minimum** `levenshtein_distance(word, needle)` across words. Matching per-word (not against the whole title) is what lets a typo in one word of a multi-word title still hit — comparing `"logn"` against the full string `"login bug"` would be distance 5 and miss.
- Keep tickets whose best distance `<= max_distance`, sort ascending by distance, return.

It's a pure function over an already group-scoped `Vec`, so the fuzzy match can never widen
the tenant boundary — the boundary was applied by the query that produced its input.

#### Inline tests (4)
`search_by_title_prefers_substring_match`, `_is_case_insensitive`,
`_falls_back_to_typo_tolerant_match` (`"logn"` → `"Login bug"`),
`_returns_nothing_when_too_dissimilar` (`"zzzzzzzz"` → empty).
A `ticket_with_title` helper builds fixtures.

---

## `ticket/handlers.rs`

### Constant
`MAX_TITLE_LEN: usize = 200`. Enforced on **`title.len()`** — that's bytes, not characters,
so a title of multi-byte characters is capped shorter than 200 visible characters. The
blank check uses `title.trim().is_empty()`.

### Helpers
- `fn parse_id(raw) -> Result<ObjectId, ApiError>` — same as the group module's.
- `fn validate_create(input)` — title non-blank and ≤ 200; description non-blank. No description length cap.
- `fn validate_update(input)` — rejects an all-`None` body with `Validation("at least one field is required")`; then, for each field **that is present**, applies the same non-blank/length rules.

### Handlers

All five take `GroupScoped`, so membership is verified before any of this runs.

| Handler | Extra extractors | Flow |
|---|---|---|
| `create_ticket` | `Json<CreateTicketRequest>` | `validate_create` → `create_ticket` → `201` |
| `list_tickets` | `Query<ListTicketsQuery>` | straight to `list_tickets` (no handler-level validation — serde already rejected bad enums) → `200` |
| `get_ticket` | `Path<(String,String)>` | discard element 0, `parse_id(ticket_id)` → `get_ticket` → `200` |
| `update_ticket` | `Path<(String,String)>`, `Json<UpdateTicketRequest>` | `parse_id` → `validate_update` → `update_ticket` → `200` |
| `delete_ticket` | `Path<(String,String)>` | `parse_id` → `delete_ticket` → `204` |

The repeated `let (_, ticket_id) = path.into_inner();` is the pattern noted in
`03-rbac-and-middleware.md`: `web::Path` extracts both segments, but the group id is taken
from `scoped.group_id` so there's exactly one source of truth for tenant scope.

---

## Test coverage

`backend/tests/ticket_api_tests.rs` (463 lines) is the integration suite for this module —
end-to-end HTTP tests against a live Mongo. Note there are no `ticket_service_tests.rs` or
`ticket_repository_tests.rs` files, unlike the group and admin modules which have all three
tiers. Ticket unit coverage lives inline (`search_by_title`, the enum serialization tests).
