# Frontend — Teams (Groups)

Covers: `services/groups.service.js`, `hooks/useGroup.js`,
`pages/GroupManagementPage.jsx`, `features/groups/` (4 components).

Reminder on vocabulary: the API, routes, and query keys all say **group**; every user-facing
string says **Team**. Intentional, not drift.

---

## `services/groups.service.js` (47 lines)

Nine thin wrappers, the complete group API surface the frontend uses:

| Function | Call | Returns |
|---|---|---|
| `listGroups()` | `GET /groups` | `GroupSummaryResponse[]` — includes `role`, `member_count`, `open_ticket_count` |
| `createGroup(name)` | `POST /groups` | `GroupResponse` |
| `getGroup(groupId)` | `GET /groups/:id` | `GroupResponse` |
| `renameGroup(groupId, name)` | `PATCH /groups/:id` | `GroupResponse` |
| `deleteGroup(groupId)` | `DELETE /groups/:id` | — (204) |
| `listMembers(groupId)` | `GET /groups/:id/users` | `MemberResponse[]` |
| `lookupUserByEmail(groupId, email)` | `GET /groups/:id/users/lookup?email=` | `{id, name, email}` |
| `addMember(groupId, userId, role)` | `POST /groups/:id/users` | `MemberResponse` |
| `updateMemberRole(groupId, userId, role)` | `PATCH /groups/:id/users/:user_id` | `MemberResponse` |
| `removeMember(groupId, userId)` | `DELETE /groups/:id/users/:user_id` | — (204) |

`addMember` maps its JS arg `userId` to the body key `user_id`, matching the Rust
`AddMemberRequest`. `removeMember` serves **both** removing someone else and leaving
yourself — the backend branches on whether the target is the caller.

---

## `hooks/useGroup.js` (29 lines)

```js
const groupQuery   = useQuery({ queryKey: ['group', groupId],            queryFn: () => getGroup(groupId) });
const membersQuery = useQuery({ queryKey: ['group', groupId, 'members'], queryFn: () => listMembers(groupId) });
```

Two **parallel** queries under a shared `['group', groupId]` prefix. The nesting is
deliberate: member mutations invalidate the narrower `['group', groupId, 'members']` key, so
the members list refetches while the group metadata query is untouched.

Collapses both into one `status`:
```js
isPending(either) → 'pending'
isError(either)   → 'error'
otherwise         → 'success'
```
Returns `{ group, members, status }` with `?? null` / `?? []` fallbacks, so consumers never
handle `undefined`.

---

## `pages/GroupManagementPage.jsx` (200 lines)

The largest page component. Route `/groups/:id`.

### Setup
`useParams()` → `id`; `useNavigate()`; `useAuth()` → `user`; `useQueryClient()`;
`useGroup(id)` → `{group, members, status}`.

Six pieces of local state, in three pairs — delete (`isConfirmingDelete`, `deleteError`,
`isDeleting`), rename (`isRenaming`), leave (`isConfirmingLeave`, `leaveError`, `isLeaving`).

### Early returns
`status === 'pending'` → "Loading…". `status === 'error'` → *"Couldn't load this team. You
may not be a member, or it may not exist."* — worded that way because the backend returns
**403 for both cases deliberately**, so the frontend genuinely cannot distinguish them.

### Role derivation
```js
const myRole = members.find(m => m.user_id === user.id)?.role;
const iAmAdmin = isGroupAdmin(myRole);
```
Derived from the members array it already has, rather than a separate call. (`GET /groups`
also carries `role` per group, but this page doesn't fetch that list.)

`iAmAdmin` drives the header: admins see a rename pencil + "Delete team"; non-admins see
"Leave team".

### Handlers

| Function | Flow |
|---|---|
| `handleDelete()` | `deleteGroup(id)` → invalidate `['groups']` (sidebar list) → `navigate('/dashboard')`. On error: set message, clear `isDeleting`. Note it deliberately does **not** clear `isDeleting` on success — the component is unmounting. |
| `handleRenamed()` | Close the modal → invalidate `['group', id]` (the heading) **and** `['groups']` (the sidebar, which shows the name too). |
| `handleLeave()` | `removeMember(id, user.id)` — same endpoint as removal, self-targeted → invalidate `['groups']` → `navigate('/dashboard')`. |

Both destructive handlers surface a sole-Group-Admin `409` through `errorMessage`, so the
backend's *"a successor Group Admin must be appointed before…"* text reaches the user verbatim.

### Render
Header (name + "Your role: Team Admin/Contributor" + action buttons) → `<GroupStats />` →
three `<Modal>`s (rename, leave-confirm, delete-confirm) → "Members" + `<MemberManager />`.

The modals are always mounted with an `isOpen` prop rather than conditionally rendered, so
Radix handles enter/exit itself.

---

## `features/groups/CreateGroupForm.jsx` (47 lines)

`useState` for `name`, `error`, `isSubmitting`. `handleSubmit` → `createGroup(name)` →
clear the field → `onCreated(group)` (the parent decides what happens next).

Deliberately **not** a React Query mutation — it doesn't own the `['groups']` cache, its
caller does. Used from the sidebar's create-team modal.

## `features/groups/RenameGroupForm.jsx` (50 lines)

Seeded with `currentName`. Computes `trimmed` and `unchanged`, and disables submit when
`isSubmitting || trimmed === '' || unchanged` — so you can't fire a no-op rename. Calls
`onRenamed()` (no argument; the parent invalidates and refetches). `autoFocus` on the input,
since it always opens inside a modal.

## `features/groups/GroupStats.jsx` (38 lines)

Three stat tiles: Members, Open Issues, Last Activity.

The clever bit — and its comment explains it — is that **open-issue count comes from
`GET /groups`, not a per-group endpoint**:
```js
const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });
const summary = groups.find(g => g.id === groupId);
```
Reasoning: `GET /groups` already reports `open_ticket_count` for every team the caller
belongs to, and viewing a team requires membership, so this team is guaranteed to be in that
list. Because it reuses the **same `['groups']` key the sidebar uses**, React Query dedupes
it — no extra network request at all.

`memberCount` is passed in as a prop from the page (which already has the array).
"Last Activity" is hardcoded `'—'` with a comment noting there's no activity tracking in the
schema. Icons come from `lucide-react` (`User`, `Ticket`, `Clock`).

---

## `features/groups/MemberManager.jsx` (222 lines)

Three components in one file.

### `AddMemberForm({ groupId })` (inner)

A two-step flow, because there is no user directory:

**Step 1 — lookup.** `handleLookup` → `lookupUserByEmail(groupId, email)` → store in `found`.
Failure shows *"No user found with that email."*

**Step 2 — confirm with a role.** When `found` is set, renders the name/email plus two
buttons: "Add as Contributor" and "Add as Team Admin". Each calls `handleConfirm(role)` →
`addMutation.mutate({userId: found.id, role})`.

`addMutation` is a React Query `useMutation`; `onSuccess` invalidates
`['group', groupId, 'members']` and resets both `found` and `email`.

`isBusy = isLookingUp || addMutation.isPending` disables the whole flow during either phase.

Showing the resolved name/email before committing matters here — you're adding by exact
email with no autocomplete, so this is the only confirmation that you got the right person.

### `MemberActionsMenu({ member, isSelf, canChangeRole, isBusy, onToggleRole, onRemove })` (inner)

A Radix `DropdownMenu` behind a `MoreVertical` trigger. Two items:
- Promote/Demote — label flips on `isGroupAdmin(member.role)`; rendered only when `canChangeRole`.
- Remove/Leave — label flips on `isSelf`; styled red.

Rendered in a `Portal`, so it escapes any parent overflow clipping.

### `MemberManager({ groupId, members, myUserId, myRole })` (default export)

Two mutations, both invalidating `['group', groupId, 'members']` on success:

| Mutation | Notes |
|---|---|
| `roleMutation` | `updateMemberRole(groupId, userId, role)` |
| `removeMutation` | `removeMember(groupId, userId)`. **`onSuccess` checks `userId === myUserId`** — if you removed yourself you've lost access, so it navigates to `/dashboard` and skips the invalidation (refetching a list you can no longer read would 403). |

`busyId` (a single user id, not a boolean) tracks which row is mid-mutation, so only that
row's menu disables. Both mutations clear it in `onSettled`.

`handleRoleToggle(member)` computes the opposite role and fires — promote/demote is a
toggle, not a picker.

**The whole actions menu and the entire add-member section render only when `iAmAdmin`.**
Cosmetic: the backend rejects both regardless.

Every member row shows a `Badge` reading "Team Admin" or "Contributor".

---

## Where team data is invalidated

Worth memorizing, since it's spread across files:

| Action | Invalidates |
|---|---|
| Create team (sidebar) | `['groups']` |
| Rename team | `['group', id]` + `['groups']` |
| Delete team | `['groups']` |
| Leave team (page button) | `['groups']` |
| Add / remove / change role | `['group', groupId, 'members']` |
| Remove **self** | *nothing* — navigates away instead |

`['groups']` is shared by `Sidebar`'s `TeamsSection` and `GroupStats`, so invalidating it
refreshes both.
