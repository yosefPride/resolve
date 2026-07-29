# Frontend — Issues (Tickets) and Comments

Covers: `services/tickets.service.js`, `hooks/useTickets.js`, `pages/TicketsPage.jsx`,
`pages/TicketDetailPage.jsx`, `features/tickets/` (6 components),
`services/comments.service.js`, `hooks/useComments.js`, `features/comments/` (2 components),
and `features/dashboard/DashboardStats.jsx`.

Vocabulary reminder, same shape as Teams/Groups: the API, routes, and query keys all say
**ticket**; every user-facing string says **Issue**. Intentional, not drift.

---

## The `?group=` convention

Worth reading before anything else, because both pages depend on it.

There is no "active group" anywhere in the frontend — that rule comes from the backend, where
scope is always an explicit `{id}` path segment. Ticket pages still need *a* group, so they
carry it in the **query string**:

```
/tickets?group=<groupId>
/tickets/<ticketId>?group=<groupId>
```

Consequences, all deliberate:

- A refresh or a shared link points at the same team's issues; nothing is held in module state or context.
- `TicketsPage` defaults to the user's **first team** when `?group=` is absent, via a `useEffect` that calls `setSearchParams({...}, { replace: true })` — `replace` so the bare `/tickets` URL doesn't become a back-button trap.
- `TicketDetailPage` does **not** default. Without `?group=` it renders "Missing team context. Open this issue from the Issues list instead." and a link back. It can't guess: a ticket id alone doesn't identify its group, and the API has no lookup that would resolve one without already knowing the other.

That last point is the practical limitation of this design — **`/tickets/<id>` is not
independently deep-linkable**. Every internal link (`TicketCard`, `DashboardStats`, the
back-links) appends `?group=`, so it only bites on a hand-typed or truncated URL.

---

## `services/tickets.service.js` (36 lines)

Five thin wrappers, the complete ticket API surface:

| Function | Call | Notes |
|---|---|---|
| `listTickets(groupId, filters)` | `GET /groups/:id/tickets` | Builds `params` conditionally |
| `createTicket(groupId, {title, description, priority})` | `POST /groups/:id/tickets` | |
| `getTicket(groupId, ticketId)` | `GET /groups/:id/tickets/:ticketId` | |
| `updateTicket(groupId, ticketId, changes)` | `PATCH /groups/:id/tickets/:ticketId` | At least one field; the backend rejects an empty body |
| `deleteTicket(groupId, ticketId)` | `DELETE /groups/:id/tickets/:ticketId` | |

`listTickets` **omits empty keys entirely** rather than sending `?status=`, so the backend
applies its own defaults (page 1, `per_page` 20). `q` is `trim()`ed and dropped when blank,
which matches how `admin.service.js` handles its `search` param. `perPage` is the one
camelCase→snake_case rename (`per_page`) done here rather than at the call site.

---

## `hooks/useTickets.js` (65 lines)

Two queries and three mutations.

### Queries
- `useTicketList(groupId, filters)` → `['tickets', groupId, filters]`, with **`placeholderData: keepPreviousData`**. The whole filter object is part of the key, so every filter/page change is its own cache entry; `keepPreviousData` keeps the current rows on screen while the next set loads instead of flashing a spinner. Same pattern as `UsersPanel`'s search.
- `useTicket(groupId, ticketId)` → `['ticket', groupId, ticketId]`. Separate key from the list, not a `select` off it — the detail page is reachable directly and can't assume the list was ever fetched.

Both are `enabled`-gated on their ids being truthy, so neither fires while `?group=` is still
being resolved.

### Mutations
All three invalidate **`['groups']`** alongside their own keys. That's the cross-feature link:
`GET /groups` reports `open_ticket_count`, which the sidebar and `GroupStats` render, so
opening, closing, or deleting a ticket makes those numbers stale.

`useUpdateTicket` additionally calls `setQueryData(['ticket', groupId, ticketId], ticket)` —
writing the server's response straight into the detail cache so the page updates without a
refetch, while the *list* is invalidated rather than patched (its filter set may no longer
match the edited ticket).

Contrast [`useComments.js`](#hooksusecommentsjs-36-lines), which deliberately does none of
this.

---

## `pages/TicketsPage.jsx` (112 lines)

Owns team selection and the create-issue modal; delegates everything about the list itself to
`TicketList`.

Reads `['groups']` directly (not through `useGroup`) because it needs the *whole* list for
the switcher dropdown. Four render states before the list: pending, error, **no teams at all**
("You're not in any team yet. Create one from the sidebar…"), and the normal case.

The switcher is a hand-rolled `absolute`-positioned dropdown toggled by `isSwitcherOpen`, not
the Radix `DropdownMenu` used by `UserMenu` — so it has no outside-click or Escape handling;
selecting a team is what closes it.

**The subtle bit:**

```jsx
{groupId && <TicketList key={groupId} groupId={groupId} />}
```

`key={groupId}` forces a full remount on team switch. Without it, `useTicketList`'s
`keepPreviousData` would render the *previous* team's tickets under the new team's heading
while the new query resolves — the one place where that otherwise-desirable behavior would
show another tenant's data in the wrong frame. Remounting also resets `TicketList`'s filter
state, which is what you want when changing teams.

---

## `features/tickets/TicketList.jsx` (98 lines)

Owns all filter state (`search`, `status`, `priority`, `creator`, `page`) and pagination.
`PER_PAGE = 20`, matching the backend's default.

`search` is debounced 300 ms via
[`useDebouncedValue`](./03-admin.md#hooksusedebouncedvaluejs-14-lines); the other three are
applied immediately, since they're `<select>`s where every change is deliberate.

```js
function updateFilter(setter) {
  return (value) => { setter(value); setPage(1); };
}
```

Every filter setter is wrapped so **any filter change resets to page 1** — otherwise
narrowing a filter can strand the view on a page number past the new `totalPages`, showing an
empty list with no obvious cause.

`totalPages` is `Math.max(1, Math.ceil(total / PER_PAGE))`, so an empty result still reports
"Page 1 of 1" rather than "of 0". The pager renders only when `totalPages > 1`.

It pulls `members` from `useGroup(groupId)` purely to hand `TicketFilters` the creator list —
a second query on the page, deduped by React Query against whatever else holds
`['group', id, 'members']`.

---

## `features/tickets/TicketFilters.jsx` (66 lines)

Pure presentational: one `Input` and three `<select>`s, every one a controlled component with
an `aria-label` (there are no visible labels). Option values are the **backend's enum
strings** (`open`/`closed`, `low`/`high`/`critical`), with `""` as the "no filter" option that
`listTickets` then drops.

There is no `medium` priority, matching `TicketPriority` — see
[`../backend/05-tickets.md`](../backend/05-tickets.md).

## `features/tickets/TicketCard.jsx` (36 lines)

A `Link` to `/tickets/:id?group=:groupId` rendering `#ticket_number`, a truncated title, the
creator's name (hidden below `sm`), and status + priority `Badge`s.

`PRIORITY_VARIANT` / `STATUS_VARIANT` map enum strings to badge variants. Both maps and the
`capitalize` helper are **duplicated verbatim in `TicketDetail.jsx`** — small, but they're the
one piece of this feature that would drift if a priority were ever added.

---

## `features/tickets/CreateTicketForm.jsx` (78 lines)

Plain `useState` + `async` handler, the convention for one-shot forms. `title`, `description`,
`priority` (defaulting to `low`); `status` and `ticket_number` are server-assigned and not
offered.

Validation is native: `required` on both text fields plus `maxLength={200}` on the title.
Note that mirrors the backend's *byte* cap approximately, not exactly —
`maxLength` counts UTF-16 units, `String::len()` counts bytes, and they agree only for ASCII.
For a title made of non-Latin characters the server rejects before the client stops you.
(`CommentForm` handles the analogous problem properly; see below.)

Clears all three fields on success, then calls `onCreated(ticket)`.

## `features/tickets/EditTicketForm.jsx` (95 lines)

Same shape, seeded from the existing ticket, plus a **Status** select — there is no separate
status endpoint, so open/closed is edited through the same `PATCH`.

It always sends **all four fields**, changed or not. Harmless given the backend requires only
"at least one" and always refreshes `updated_at` regardless, but it means an edit that changed
nothing still bumps the timestamp.

Rendered only for Group Admins (`TicketDetail` gates it), which matches the backend: not even
the ticket's creator may edit it.

---

## `pages/TicketDetailPage.jsx` (73 lines)

Resolves three things before rendering: the ticket (`useTicket`), the group's members
(`useGroup`, for the caller's role), and `groupId` from `?group=`. Pending and error states
are combined across both queries; the error copy is deliberately ambiguous — *"It may not
exist, or you may not have access."* — because the backend returns `404` for both a missing
ticket and one in another group, and the UI shouldn't invent a distinction the API refuses to
make.

Derives `myRole` by finding the caller in the members array, then passes
`isAdmin={isGroupAdmin(myRole)}` down. Same approach as `GroupManagementPage`.

## `features/tickets/TicketDetail.jsx` (154 lines)

The page body: header (`#number`, title, status + priority badges), the
opened-by/updated-at line, description, an admin action row, then the Comments and AI
sections.

`updated_at` is shown only when it differs from `created_at`, so a never-edited ticket doesn't
display two identical timestamps.

The admin row (Close/Reopen, Edit, Delete) renders only when `isAdmin`. **UX only** — the
backend rejects a non-Group-Admin `PATCH`/`DELETE` regardless. Close/Reopen is a one-field
`PATCH` through the same `useUpdateTicket`; delete confirms in a `Modal`, then navigates back
to `/tickets?group=…` on success.

The **AI section is a "Coming soon." placeholder** holding the layout slot
`docs/specification/frontend.md` describes. It is the last unbuilt piece of this page.

---

## `services/comments.service.js` (26 lines)

Three wrappers. `listComments` returns the **whole thread, flat and oldest-first** — comments
are not paginated the way tickets are. `createComment` translates `parentCommentId` →
`parent_comment_id`, sending `null` for a top-level comment.

## `hooks/useComments.js` (36 lines)

`useComments` → `['comments', groupId, ticketId]`; `useCreateComment` and `useDeleteComment`
both invalidate exactly that key.

Two deliberate differences from `useTickets`:

- **Nothing touches `['groups']`.** A comment doesn't change `open_ticket_count`, so the sidebar and `GroupStats` numbers can't go stale from one.
- **Delete refetches rather than removing the comment from the cache.** The backend chooses between a hard delete (leaf) and a tombstone (comment with replies, which stays in the list with `is_deleted` set); reproducing that rule client-side to predict the outcome would be a second copy of it to keep in sync.

## `features/comments/CommentList.jsx` (264 lines)

The largest component in the frontend, and the only one that builds a tree.

### `buildCommentTree(comments)`
One pass over the flat array: a `Map` of id → node (each given a `replies: []`), then each
node pushed onto its parent's `replies` or onto `roots`. `Map` preserves insertion order, so
roots and every reply list come out oldest-first **with no sorting** — the API's ordering is
inherited for free.

A reply whose parent isn't in the payload is **promoted to a root** rather than dropped. That
shouldn't normally happen, since a comment with replies is tombstoned instead of removed — but
the backend's `has_replies` check and the delete that follows it aren't atomic, so a reply
created in that window can point at an id that's already gone. It renders with an "In reply to
a comment that was deleted" marker instead of vanishing.

### Rendering
`CommentItem` recurses. Two things carry the thread structure:

- **Indentation**, capped at `MAX_INDENT_DEPTH = 4`. Deeper replies keep their real depth in the tree but stop indenting, so a long back-and-forth can't squeeze the text column to nothing on a phone.
- **`QuotedParent`**, a WhatsApp-style header showing the parent's author and first line (`line-clamp-1`). Rendered at **every** depth, not only past the cap — it's what makes a reply readable once indentation stops growing.

`canDelete` is `!is_deleted && (own comment || isAdmin)`, mirroring
`RbacService::require_owner_or_group_admin`. UX only; and a tombstone has nothing left to
delete.

Comment bodies render as **plain text through JSX**, never `dangerouslySetInnerHTML`. The
backend stores content verbatim without sanitising it, so React's automatic escaping is the
only thing between a pasted `<script>` and the DOM. The file carries an explicit comment
saying not to swap this for a markdown renderer.

### Closed tickets
`isClosed` removes the composer outright rather than disabling it — a closed ticket answers
`POST` with `409`, so there's nothing useful to click. If someone else closes the ticket while
a reply box is open, `activeReplyingTo` forces it shut on the next render rather than leaving
a form that can only fail.

## `features/comments/CommentForm.jsx` (92 lines)

Serves both roles: top-level composer when `parentCommentId` is `null`, inline reply box when
it's set (fewer rows, `autoFocus`, a Cancel button, "Reply" instead of "Comment").

```js
function contentLength(value) { return [...value].length; }
```

Counts **Unicode code points**, matching the backend's `.chars().count()`. Plain
`value.length` counts UTF-16 units, and the two disagree outside the BMP — `'😀'.length` is 2
where the backend counts 1. Same reason the textarea carries **no `maxLength`**: it would
truncate emoji-heavy input well before the real 2000 limit. The live counter turns red past
the cap and the submit button disables, so the constraint is enforced without cutting anyone
off mid-word.

A `409` on submit gets its own message — *"This issue was closed. Reload the page to see its
current state."* — rather than the generic fallback, which would otherwise leave someone
staring at a visible composer that permanently fails with no explanation.

---

## `features/dashboard/DashboardStats.jsx` (77 lines)

What `DashboardPage` renders below the greeting: two summary tiles (team count, total open
issues) and a card per team linking to `/tickets?group=<id>`.

**No dedicated dashboard endpoint exists or is needed.** `GET /groups` already returns
`member_count`, `open_ticket_count`, and `role` per team, so this reuses the **same
`['groups']` key** as the sidebar and `GroupStats` — usually a cache hit, and automatically in
sync with any create/rename/delete those views invalidate. The total is a `reduce` over
`open_ticket_count` client-side.

Handles the empty case with its own create-team modal rather than pointing at the sidebar, so
a brand-new account has an action on the first screen it lands on.

Tiles come from `components/ui/StatTile.jsx`, extracted from `GroupStats` when this was built
so the two don't carry duplicate markup — see
[`05-layout-and-ui.md`](./05-layout-and-ui.md).
