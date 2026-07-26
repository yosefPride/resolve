# Frontend — Admin Panel

Covers: `services/admin.service.js`, `pages/AdminPage.jsx`, `features/admin/` (4 components),
`features/users/UserTable.jsx`, `hooks/useDebouncedValue.js`.

Reachable only at `/admin`, guarded by `AdminRoute` (auth + `isSystemAdmin`). The backend
re-checks every call via the `SystemAdminUser` extractor, so this gate is UX-only.

---

## `services/admin.service.js` (47 lines)

| Function | Call | Notes |
|---|---|---|
| `listUsers(search)` | `GET /admin/users` | Sends `?search=` only when non-empty after `trim()`; otherwise omits the param entirely |
| `listGroups(search)` | `GET /admin/groups` | Same trim-and-omit pattern |
| `deleteGroup(groupId)` | `DELETE /admin/groups/:id` | |
| `deletionCheck(userId)` | `GET /admin/users/:id/deletion-check` | Returns `{blocked_groups, auto_delete_groups}` |
| `deleteUser(userId, successors)` | `POST /admin/users/:id/delete` | `successors` is `{[group_id]: successor_user_id}` |
| `listAuditLog(filters = {})` | `GET /admin/audit-log` | Maps `filters.groupId` → `group_id`, `filters.userId` → `user_id`; omits absent keys |

The `const term = search?.trim(); if (term) params.search = term;` idiom makes a
whitespace-only search behave exactly like no search — matching the backend handler, which
does the same trim-and-drop on its side.

Note the file's header comment draws the boundary: these are the System-Admin-only
endpoints; non-admin user/group operations live in `groups.service.js`.

---

## `hooks/useDebouncedValue.js` (14 lines)

```js
useEffect(() => {
  const timer = setTimeout(() => setDebounced(value), delay);
  return () => clearTimeout(timer);
}, [value, delay]);
```

Returns `value` once it has stopped changing for `delay` ms (default 300). The cleanup
cancels the pending timer on every keystroke, so only the final pause fires. Used by both
admin search boxes to keep the debounced value out of the React Query key until typing settles.

---

## `pages/AdminPage.jsx` (48 lines)

A tab shell. `const [activeTab, setActiveTab] = useState('users')` over:

```js
const TABS = [
  { id: 'users',  label: 'Users' },
  { id: 'groups', label: 'Teams' },     // "Teams" in the UI, /admin/groups in the API
  { id: 'audit',  label: 'Audit Log' },
];
```

Renders `role="tablist"` / `role="tab"` / `aria-selected` / `role="tabpanel"`, then
conditionally mounts `<UsersPanel />`, `<GroupsPanel />`, or `<AuditLogPanel />`.

Because tabs **unmount** when switched away, each panel's queries go inactive — but React
Query keeps the cached data, so switching back shows previous results immediately while
refetching in the background.

---

## `features/admin/UsersPanel.jsx` (64 lines)

### Search + query
```js
const [search, setSearch] = useState('');
const debouncedSearch = useDebouncedValue(search, 300);

const { data: users = [], status } = useQuery({
  queryKey: ['admin', 'users', debouncedSearch],
  queryFn: () => listUsers(debouncedSearch),
  placeholderData: keepPreviousData,
});
```

Two things to note:
- **The debounced value is part of the query key**, so each distinct search term is cached separately and re-typing a previous term is instant.
- **`placeholderData: keepPreviousData`** keeps the current rows on screen while a new term fetches, so typing swaps results in place instead of flashing a spinner. The comment says this replaced a hand-rolled "don't reset status while typing" workaround — a good example of a library feature retiring custom code.

### `handleDeleted()`
Closes the modal and invalidates **both** `['admin', 'users']` and `['admin', 'groups']` —
because a user deletion can cascade into auto-deleted groups, so the teams tab may now be stale.

Note these invalidations use the *prefix* `['admin', 'users']`, which matches every cached
search term, not just the current one.

### Render
Search `Input` → `pending` / `error` / `success` branch. On success with zero rows, the
empty message is term-aware: *"No users match "foo""* vs *"No users found."*. Otherwise
`<UserTable>`, plus `<DeleteUserModal>` when `deleteTarget` is set.

---

## `features/users/UserTable.jsx` (49 lines)

Purely presentational — loading and error states live in the parent panel.

Columns: Name, Email, Global Role (`isSystemAdmin(user) ? 'System Admin' : 'User'`), Created
(`formatDate`), Actions.

The caller's own row renders the text "You" instead of a Delete button. **The file's comment
claims "backend rejects self-deletion anyway" — that is not true;** `AdminService::delete_user`
has no such check. See [`../deviations.md`](../deviations.md).

---

## `features/admin/GroupsPanel.jsx` (120 lines)

Same search/debounce/`keepPreviousData` pattern as `UsersPanel`, keyed
`['admin', 'groups', debouncedSearch]`.

### Delete flow
```js
const deleteMutation = useMutation({
  mutationFn: (id) => deleteGroup(id),
  onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['admin','groups'] }); setTarget(null); },
  onError: (err) => setDeleteError(errorMessage(err, 'Failed to delete team.')),
});
```

`target` holds the group pending deletion (or `null`), which doubles as the modal's
`isOpen={!!target}`.

`closeModal()` **early-returns when `deleteMutation.isPending`**, so you can't dismiss the
dialog mid-flight and lose the result. Both the Cancel and Delete buttons are disabled while
pending, and the Delete button's label switches to "Deleting…".

### Table
Name, Created, Actions. **Metadata only** — no member count, no ticket count. That's group
isolation showing up in the UI: the admin genuinely cannot see inside a group they're not a
member of, and the backend's `GET /admin/groups` returns nothing more.

---

## `features/admin/AuditLogPanel.jsx` (121 lines)

### `distinctBy(entries, idKey, nameKey)`
```js
const seen = new Map();
for (const entry of entries) if (!seen.has(entry[idKey])) seen.set(entry[idKey], entry[nameKey]);
return [...seen.entries()].map(([id, name]) => ({ id, name }));
```
Distinct `{id, name}` pairs in first-seen order.

Why it exists — and this is the key insight of this component: **the log carries snapshotted
names**, so building filter options from the log itself covers entities that no longer exist
(deleted users, auto-deleted groups). A lookup against `/admin/users` or `/admin/groups`
could not, since those records are gone.

### Data + filtering
One query, `['admin', 'auditLog']`, fetched **unfiltered**. Both dropdowns then filter
**in memory**:
```js
const visible = entries.filter(e =>
  (!groupFilter || e.group_id === groupFilter) &&
  (!userFilter  || e.deleted_user_id === userFilter));
```

The comment justifies ignoring the backend's `?group_id=` / `?user_id=` params: the log is
low-volume system metadata, so filtering locally beats a round trip per dropdown change. The
`useMemo`'d option lists derive from the **full** loaded log, so they stay stable while filtering.

So `admin.service.js::listAuditLog` accepts filters that this — its only caller — never uses.

### Table
Action (as a `Badge`, via `ACTION_LABELS = {succession: 'Succession', group_auto_deleted:
'Team auto-deleted'}` with a fallback to the raw value), Team, Deleted user, Successor
(`|| '—'`, null for auto-deletions), Performed by, When (`formatDateTime`).

Every name column reads a `*_name` snapshot field, never a resolved id.

---

## `features/admin/DeleteUserModal.jsx` (174 lines)

The most complex component in the app. It mirrors the backend's plan-then-commit shape exactly.

### The two-call flow
```
GET  /admin/users/:id/deletion-check   → classify the target's groups
POST /admin/users/:id/delete           → commit, resolving succession
```

```js
const deletionQuery = useQuery({
  queryKey: ['admin', 'deletionCheck', user.id],
  queryFn: () => deletionCheck(user.id),
});
const blocked    = deletionQuery.data?.blocked_groups ?? [];
const autoDelete = deletionQuery.data?.auto_delete_groups ?? [];
```

### `chosenSuccessor(group)` — the important function
```js
const chosen = successors[group.group_id];
return group.eligible_successors.some(m => m.user_id === chosen) ? chosen : '';
```

Returns a stored pick **only if that person is still eligible**. After a 409 re-check the
plan can change, so a previously valid choice may not be anymore.

The comment spells out why this is derived rather than stored: pruning `successors` state in
a `useEffect` would mean syncing two sources of truth. Deriving keeps both the submit
payload and the button's enabled state correct with no synchronization at all.

`allSuccessorsChosen = blocked.every(g => chosenSuccessor(g))` gates the submit button.

### `deleteMutation`
`mutationFn` rebuilds the payload from `chosenSuccessor` (not raw state), so an invalidated
pick is never sent.

`onError` branches on status:
```js
if (err.response?.status === 409) {
  setSubmitError('These teams changed since the last check. Please review and try again.');
  deletionQuery.refetch();      // re-run the check in place
} else { setSubmitError(errorMessage(err, 'Failed to delete user.')); }
```
A 409 means the server re-derived a different plan (membership shifted between check and
commit). The modal stays open, refetches, and lets the admin re-choose.

### `checkStatus` — three-way, deliberately not React Query's own status
```js
!deletionQuery.isSuccess && deletionQuery.isFetching ? 'loading'
  : deletionQuery.isError ? 'error' : 'ready'
```
The `!isSuccess &&` is what makes the 409 path work: a **background re-check of an
already-loaded plan** stays `'ready'`, so the form doesn't flicker back to a spinner. Only
the initial check (or a retry after error) shows "Checking teams…".

### Render branches
- `'loading'` → "Checking teams…"
- `'error'` → message + Cancel / Retry (`deletionQuery.refetch()`)
- `'ready'` → three sub-cases:
  - **No blockers** → plain confirmation text.
  - **`blocked` groups** → a card per group with a `<select>` of `eligible_successors` (each option showing name + email), labeled "Promote a member to Team Admin".
  - **`autoDelete` groups** → a red-bordered warning card: "This team has no other members and will be deleted."

`closeIfIdle()` early-returns while the mutation is pending, so the modal can't be dismissed
mid-delete.

---

## Admin query keys

| Key | Owner | Invalidated by |
|---|---|---|
| `['admin', 'users', search]` | `UsersPanel` | user deletion |
| `['admin', 'groups', search]` | `GroupsPanel` | group deletion, **and** user deletion (cascade) |
| `['admin', 'auditLog']` | `AuditLogPanel` | *nothing* — see below |
| `['admin', 'deletionCheck', userId]` | `DeleteUserModal` | its own `refetch()` on 409 |

Note the gap: deleting a user writes audit entries, but nothing invalidates
`['admin', 'auditLog']`. The admin must switch tabs (which remounts the panel and triggers a
background refetch) to see new entries. Minor, and self-correcting in practice.
