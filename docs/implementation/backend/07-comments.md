# Backend — Comments

Covers `src/comment/`: `models.rs` (50), `repository.rs` (170), `service.rs` (174),
`handlers.rs` (129), `mod.rs` (4).

The smallest feature module, and the only one with a **self-referential** entity: a comment
may point at another comment on the same ticket, so the module's interesting parts are all
consequences of that — unlimited-depth threading, a tombstone-vs-hard-delete decision on
every delete, and two bulk cascade deletes that other modules call into.

Routes are nested one level deeper than tickets
(`/groups/{id}/tickets/{ticket_id}/comments/...`), registered in `server/routes.rs` inside
the same group scope, so `GroupScoped` can still read `{id}`.

---

## `comment/models.rs`

### `struct Comment`
Stored document: `id`, `group_id`, `ticket_id`, `parent_comment_id: Option<ObjectId>`,
`user_id`, `content`, `is_deleted: bool`, `created_at`.

- **`parent_comment_id` is what makes threading work** and it carries no depth limit — it names *any* comment on the same ticket, including one that is itself a reply. `None` means top-level on the ticket.
- **`group_id` is stored on the comment itself**, not inferred through the ticket. Redundant with `ticket.group_id`, and deliberately so: it's what lets every comment query filter on `group_id` directly (the isolation pattern the rest of the backend uses) and what lets `delete_by_group` be one query instead of a per-ticket fan-out.
- There is **no `updated_at`**, because there is no edit path — a comment can only be created and deleted.

### `struct CreateCommentInput { group_id, ticket_id, parent_comment_id, user_id, content }`
Repository input. `is_deleted` and `created_at` are absent — the repository always sets
`false` and `now`.

### `const DELETED_CONTENT_PLACEHOLDER: &str = "[comment deleted]"`
The text a tombstoned comment's `content` is replaced with. Written by
`CommentRepository::soft_delete`, and the string clients actually render for a deleted
comment that still has replies.

### `struct CreateCommentRequest { content, parent_comment_id: Option<String> }`
Body of `POST`. `user_id`, `is_deleted`, and `created_at` are server-assigned and **cannot
be supplied by the client** — they aren't fields on this struct. `parent_comment_id` arrives
as a `String` and is parsed to `ObjectId` in the handler.

### `struct CommentResponse`
Client shape: comment fields with ObjectIds as hex strings and `created_at` as
`DateTime<Utc>`, **plus `user_name: String`** — the same denormalized author join
`TicketResponse` does with `created_by_name`.

Note `is_deleted` is on the wire: the list endpoint returns tombstones, so the client needs
to know which entries are one.

### Inline tests
None in this file — the model's serde shape carries no enums to validate (contrast
`ticket/models.rs`, whose four inline tests all guard `TicketStatus`/`TicketPriority`).

---

## `comment/repository.rs`

### `enum CommentRepoError { Database(_) }`
Single variant, like `TicketRepoError` — no unique constraint the app expects to trip. Maps
to `ApiError::Internal`.

### `struct CommentRepository { comments: Collection<Comment> }`
One collection: `"comments"`.

### `async fn insert_comment(&self, input) -> Result<Comment, _>`
Sets `is_deleted: false` and `created_at: now`, inserts, returns the struct with the
generated `_id`.

### `async fn find_by_id(&self, group_id, ticket_id, comment_id) -> Result<Option<Comment>, _>`
**Filters on `_id` AND `group_id` AND `ticket_id`** — the same query-shape isolation as
`TicketRepository::find_by_id`, one level deeper. A comment id from another group or another
ticket matches nothing → `None` → `404`.

### `async fn list_by_ticket(&self, group_id, ticket_id) -> Result<Vec<Comment>, _>`
`find({group_id, ticket_id}).sort({created_at: 1})`, whole cursor collected.

**No pagination** — a discussion thread is read in full, and **no server-side nesting** —
the result is flat and oldest-first, leaving the client to assemble the reply tree from each
comment's `parent_comment_id`. Tombstones are included on purpose: drop them and their
surviving replies would render against a parent that isn't in the response.

### `async fn has_replies(&self, comment_id) -> Result<bool, _>`
`count_documents({parent_comment_id: comment_id}) > 0`. This one query is what decides hard
vs. soft delete. Note it is **not** group-filtered — it's only ever called on a comment
already fetched through `find_by_id`, so the tenant check has happened; the
`parent_comment_id` index is what makes it cheap.

### `async fn hard_delete(&self, group_id, ticket_id, comment_id) -> Result<bool, _>`
`delete_one` on the full `{_id, group_id, ticket_id}` filter.

### `async fn soft_delete(&self, group_id, ticket_id, comment_id) -> Result<bool, _>`
The tombstone. `update_one` with
`$set: { is_deleted: true, content: DELETED_CONTENT_PLACEHOLDER }` — the document survives
so replies keep a valid `parent_comment_id`, but the original text is **overwritten in the
database**, not merely hidden at read time. A tombstone is not recoverable.

Returns `modified_count > 0`, so tombstoning an already-tombstoned comment reports `false`
(nothing changed) and the service turns that into `404`.

### `async fn delete_by_ticket(&self, group_id, ticket_id) -> Result<u64, _>`
Cascade target for deleting a **single ticket**. Unconditional `delete_many` — tombstones
included, since the ticket they belong to is going away and there's nothing left for a reply
to stay valid against. Filtered on `group_id` as well as `ticket_id` specifically so the
query can use the `(group_id, ticket_id)` index; `ticket_id` alone is not a prefix of it.

Called by `TicketService::delete_ticket`.

### `async fn delete_by_group(&self, group_id) -> Result<u64, _>`
Cascade target for deleting a **whole group**. Same unconditional `delete_many`, scoped to
`group_id` only — because comments carry their own `group_id`, this needs no per-ticket
fan-out. `group_id` is the prefix of the `(group_id, ticket_id)` index, so it's served too.

Called by `purge_group_data` (`group/service.rs`).

---

## `comment/service.rs`

### `struct CommentService { repo, ticket_repo, user_service, rbac }`
The `ticket_repo` is here for one reason: verifying the ticket in the path belongs to the
group in the path. See `require_ticket_in_group` below.

### `async fn create_comment(&self, user_id, group_id, ticket_id, content, parent_comment_id)`
Four checks, in this order:

1. `require_member` — **any group member may comment.** No owner/admin split on creating one, unlike tickets where editing is Group-Admin-only.
2. `require_ticket_in_group(group_id, ticket_id)` → the `Ticket`, or `404`.
3. If `ticket.status == Closed` → `ApiError::Conflict("cannot comment on a closed ticket")` → **`409`**. Conflict rather than Forbidden is deliberate: the caller has permission, it's the ticket's *state* that rejects this. Note `list_comments` is **not** gated on status and neither is `delete_comment` — a closed ticket's thread stays readable and its comments stay deletable. Reopening the ticket restores commenting.
4. If `parent_comment_id` is `Some`, `repo.find_by_id(group_id, ticket_id, parent_id)` must find it → otherwise `Validation("parent comment not found on this ticket")` → `400`. Because that lookup is filtered on both ids, a parent from a different ticket or group fails here — a reply can never dangle across a boundary.

Then `insert_comment` → `enrich_comment`.

### `async fn list_comments(&self, user_id, group_id, ticket_id) -> Result<Vec<CommentResponse>, _>`
`require_member` → `require_ticket_in_group` → `list_by_ticket` → `enrich_comment` per row.

The enrichment loop is **unbounded**: one `users` lookup per comment, over the entire thread,
with no pagination to cap it. That's the module's one real cost, and it's the same
one-lookup-per-row tradeoff `TicketService::enrich_ticket` makes — except `list_tickets`
enriches only the current page, and this enriches everything.

### `async fn delete_comment(&self, user_id, group_id, ticket_id, comment_id) -> Result<(), _>`
1. `require_member` → the caller's `GroupMember` (needed for their role in the next step).
2. `find_by_id` → `ok_or(NotFound)`.
3. `RbacService::require_owner_or_group_admin(&member, comment.user_id)` — **author or Group Admin**, else `403`. This is the one place in the backend where authorship grants a permission; tickets deliberately give their creator nothing.
4. `has_replies(comment_id)` decides the branch:
   - **has replies → `soft_delete`** (tombstone). The document stays so its replies keep a valid parent.
   - **no replies → `hard_delete`.** A leaf leaves nothing behind to protect.
5. If the chosen operation reports no change → `NotFound`.

The tombstone rule is per-comment and evaluated at delete time, so deleting a thread
bottom-up hard-deletes every comment, while deleting top-down leaves a chain of tombstones.

### `async fn require_ticket_in_group(&self, group_id, ticket_id) -> Result<Ticket, ApiError>` (private)
`ticket_repo.find_by_id(group_id, ticket_id).ok_or(NotFound)`. Called by **every** comment
read and write.

**This is the module's tenant boundary**, and it carries more weight here than the equivalent
does anywhere else. `GroupScoped` proves the caller is a member of `{id}` — it says
**nothing** about which group `{ticket_id}` belongs to. Without this check a legitimate
member of their own group could pass another group's ticket id in the path and have a comment
recorded under their own `group_id` against a ticket that isn't in it.

The two-mechanism isolation model in `backend-flow.md` §Flow D assumes the repository filter
covers the resource id. For tickets it does — `find_by_id(group_id, ticket_id)` constrains
the very id that could be foreign. For comments it doesn't: the filtered `comment.group_id`
is written from the caller's own scope, so it proves nothing about the ticket. That's the gap
this check fills, and it's why the integration suite keeps a permanent test for it.

Returning the `Ticket` (rather than `()`) is why `create_comment` gets its status check for
free instead of querying twice.

### `async fn enrich_comment(&self, comment: Comment) -> Result<CommentResponse, ApiError>` (private)
One `user_service.find_by_id(comment.user_id)` to attach `user_name`; a deleted author
yields `""` rather than an error. Mirrors `TicketService::enrich_ticket` and
`GroupService::enrich_member`.

---

## `comment/handlers.rs`

### Constant
`MAX_CONTENT_LEN: usize = 2000`. Enforced on **`content.chars().count()`** — characters, not
bytes.

Worth noting because it differs from `ticket/handlers.rs`, which caps `title.len()` — bytes.
The comment in the file spells out why characters are the right unit: a script that encodes
at 2 bytes per character (Hebrew, in the test) would otherwise be cut off at half the nominal
limit. The ticket title cap still measures bytes; see `deviations.md` §8c.

### Helpers
- `fn parse_id(raw)` — same as the group and ticket modules'.
- `fn validate_create(input)` — `content.trim()` non-blank, and `content.chars().count() <= 2000`. That's all; `parent_comment_id`'s *existence* is a service concern, only its *format* is checked here (via `parse_id`).

### Handlers

All three take `GroupScoped`, so membership is verified before any of this runs.

| Handler | Extra extractors | Flow |
|---|---|---|
| `create_comment` | `Path<(String,String)>`, `Json<CreateCommentRequest>` | `parse_id(ticket_id)` → `validate_create` → parse optional `parent_comment_id` → `create_comment` → `201` |
| `list_comments` | `Path<(String,String)>` | `parse_id(ticket_id)` → `list_comments` → `200` |
| `delete_comment` | `Path<(String,String,String)>` | `parse_id` ×2 → `delete_comment` → `204` |

The `let (_, ticket_id) = path.into_inner();` pattern is the same one described in
`03-rbac-and-middleware.md`: `web::Path` extracts every segment, but the group id is taken
from `scoped.group_id` so there's exactly one source of truth for tenant scope.

### Inline tests (3)
All on `validate_create`: rejects blank content, rejects over-limit content, and
`validate_create_accepts_hebrew_content_at_exact_char_limit` — a permanent regression guard
for the byte-vs-character bug, asserting 2000 two-byte characters pass.

---

## Cross-module wiring

Comments are the first feature other modules had to cascade into, and that reshaped two
existing delete paths:

| Caller | Calls | Why |
|---|---|---|
| `TicketService::delete_ticket` | `CommentRepository::delete_by_ticket` | one ticket's comments |
| `purge_group_data` (`group/service.rs`) | `CommentRepository::delete_by_group` | a whole group's comments |

`purge_group_data` is a free function, not a method, so all three group-destroying paths
share it: `GroupService::delete_group`, `AdminService::delete_group`, and the sole-admin
auto-delete loop in `AdminService::delete_user`. See [`04-groups.md`](./04-groups.md).

Both cascades order **child before parent** — comments, then the ticket or group — so a
mid-failure leaves the parent still resolvable and the cascade re-runnable. Sequential
writes, not transactions, consistent with the rest of the backend.

`TicketService::delete_ticket` also changed shape for this: it now does an explicit
`find_by_id` existence check **before** the cascade (so a bogus ticket id still `404`s
before any write) rather than inferring existence from `delete_ticket`'s return value.

---

## Test coverage

Three tiers, the fullest of any feature module:

- `backend/tests/comment_service_tests.rs` (571 lines) — service-level rules: threading depth, the closed-ticket lock, owner-or-admin delete, tombstone vs. hard delete, both cascades.
- `backend/tests/comment_api_tests.rs` (826 lines) — end-to-end HTTP against a live Mongo, including the cross-tenant regression test for `require_ticket_in_group`.
- Inline (3) — `validate_create`, above.

Run Mongo-backed test files with `--test-threads=1`.
