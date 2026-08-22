# Frontend — as built

How the React client is actually organized: how a session is established, how the
current team is chosen, and where each kind of code lives. For the *intended*
design see [`docs/specification/frontend.md`](../specification/frontend.md);
divergences are listed in [`deviations.md`](deviations.md).

Stack: React 19, Vite 8, React Router 7, TanStack Query 5, axios, Tailwind CSS 4
(via `@tailwindcss/vite`), Radix UI (dialog + dropdown menu), `lucide-react` for
icons, `react-markdown` + `remark-gfm` for ticket descriptions. Plain JavaScript
with JSX — no TypeScript.

The frontend is UI only. It never enforces permissions; it hides controls the
user cannot use, and the backend rejects anything that slips through.

**Naming.** The UI says "Team" wherever the backend, API, and database say
"group". This is a deliberate, UI-only rename: props, hooks, query keys, and
routes all still use `group`/`groupId`, and only user-visible strings changed.
Seeing both words in this document is expected, not a bug.

---

## Composition root

`main.jsx` nests the providers in a fixed order:

```
StrictMode
 └ QueryClientProvider   (queries retry once, not the default three)
    └ BrowserRouter
       └ AuthProvider    (blocks on session bootstrap)
          └ App
```

`AuthProvider` sits inside the router because it navigates on logout and on a
failed refresh.

## Session and the access token

The access token is held **in a module variable in `lib/axios.js`** — never in
`localStorage` or `sessionStorage`, so it does not survive a reload and is not
reachable by injected script. Durability comes from the refresh cookie, which is
`httpOnly` and set by the backend.

**Boot.** `AuthContext` calls `POST /auth/refresh` once on mount; on success it
stores the returned JWT and fetches `/auth/me`. The in-flight promise is
module-scoped rather than component state, because React StrictMode invokes
effects twice in development and refresh tokens are single-use — a genuine second
call would 401 and flip a valid session to logged out. Until it settles the app
renders a spinner, so no route ever renders in an unknown auth state. Status is
one of `loading` / `authenticated` / `unauthenticated`.

**Per request.** A request interceptor attaches `Authorization: Bearer <token>`
when one is set. `withCredentials` is on so the refresh cookie travels.

**On 401.** The response interceptor transparently refreshes and retries the
original request once, except on `/auth/login`, `/auth/register`, and
`/auth/refresh`. Concurrent 401s share a single in-flight refresh promise —
without that, the second request would arrive with an already-rotated token. If
the refresh itself fails, the token is cleared and a handler registered by
`AuthContext` resets state and navigates to `/login`.

**Response shape.** The success interceptor unwraps `response.data`, so service
functions return the parsed body directly. Errors are *not* unwrapped: they
reject with the full axios error, because call sites need `err.response`.
A 15-second timeout means a silently dropped network rejects instead of hanging
on "Loading…" forever; a timeout has no `error.response`, which is the same
shape every call site already handles.

## Routing

`App.jsx` declares two layout groups plus a bare catch-all:

| Route | Layout | Guard |
| --- | --- | --- |
| `/`, `/register`, `/login` | `MarketingLayout` | none |
| `/dashboard`, `/tickets`, `/tickets/:ticketId`, `/account`, `/groups/:id` | `AppLayout` | `ProtectedRoute` |
| `/admin` | `AppLayout` | `ProtectedRoute` + `AdminRoute` |
| `*` | its own | none |

The auth gate wraps the shared layout rather than each page, so `AppLayout`'s
chrome does not remount while navigating between app pages. `AdminRoute` stays on
the `/admin` leaf because it is an extra role check layered on top of
authentication. `NotFoundPage` sits outside both groups deliberately: it picks
its own chrome based on auth state, and a nested `*` in either group would race
the other on route ranking.

## No active group

There is no globally held "current group", mirroring the backend rule that every
group-scoped call names its group in the path. Instead:

- `/groups/:id` takes the group from the **route param**.
- `/tickets` and `/tickets/:ticketId` take it from a **`?group=<id>` query
  param**, so a link or a refresh restores exactly the same view. `TicketsPage`
  defaults to the user's first team when the param is absent and rewrites the URL
  (`replace: true`); switching teams sets the param, and `TicketList` is keyed on
  the group id so a switch remounts rather than showing stale rows.
- The dashboard spans teams by fetching each group's tickets in parallel
  (`useDashboardOverview` over `useQueries`) and merging client-side. Each
  request is still an ordinary RBAC-scoped `/groups/{id}/tickets` call — never a
  cross-group query. It caps at 100 tickets per group, so very large groups
  undercount in that widget.

## Directory layout

| Path | Contents |
| --- | --- |
| `pages/` | One component per route. Reads URL params, composes feature components, handles top-level loading/error/empty states. |
| `features/<domain>/` | The real UI for a domain: `account`, `activity`, `admin`, `ai`, `auth`, `comments`, `dashboard`, `groups`, `links`, `tickets`, `users`. |
| `components/ui/` | Presentational primitives, domain-free: `Button`, `Input`, `Textarea`, `Select`, `Field`, `Badge`, `Avatar`, `Table`, `Pagination`, `Spinner`, `StatTile`, `Modal`, `ConfirmModal`, `DropdownMenu`, `EmojiPicker`. |
| `components/layout/` | `MarketingLayout`, `AppLayout`, `Sidebar`, `Header`, `Footer`. |
| `components/marketing/` | Landing page sections plus the self-contained product demo under `marketing/demo/`. |
| `services/` | One module per API area; thin `api.get/post/...` wrappers that own URL construction. The only place endpoint paths appear. |
| `hooks/` | TanStack Query hooks and small shared behaviors. |
| `lib/` | Cross-cutting wiring: `axios.js`, `AuthContext.jsx`, `authContext.js`, `ProtectedRoute.jsx`, `AdminRoute.jsx`. |
| `utils/` | Pure helpers: `errors.js`, `format.js`, `roles.js`. |

The dependency direction is one-way: `pages → features → components/ui`, with
`hooks → services → lib/axios` underneath. UI primitives never import from
`features/`, and nothing but `services/` builds an API URL.

`lib/authContext.js` holds only `createContext(...)`; the provider lives in
`AuthContext.jsx`. They are split so the context object can be imported without
pulling in a component, which keeps Vite's fast refresh working.

## Data layer

Server state is TanStack Query; only genuinely local state (form fields, open
modals, drawer state) is React state. There is no Redux/Zustand store.

Query keys are hierarchical, so an invalidation at a prefix clears everything
below it:

| Key | Query |
| --- | --- |
| `['groups']` | The user's groups. |
| `['group', groupId]`, `['group', groupId, 'members']` | Group detail and its members. |
| `['tickets', groupId, filters]` | A filtered ticket page. |
| `['ticket', groupId, ticketId]` | One ticket. |
| `['comments' \| 'activity' \| 'links' \| 'references' \| …, groupId, ticketId]` | Ticket-scoped collections. |

Conventions worth knowing:

- Ticket list queries use `placeholderData: keepPreviousData` so filtering and
  paging swap rows in place instead of flashing a loading state. Admin search
  does the same.
- Every ticket mutation also invalidates `['groups']`, because a group's
  `open_ticket_count` (shown in the sidebar and group stats) changes with it.
- `hooks/ticketResourceHooks.js` builds the list/create/delete hook triple shared
  by ticket-scoped collections such as links and references. Resources whose
  mutations must touch other keys stay hand-written.
- `useSubmit` is the shared form choreography — clear error, run action, map a
  failure through `errorMessage`, track pending. Forms with field-level errors or
  special-cased status codes (profile, password change, comment) keep their own
  handlers rather than growing options on the hook.
- `errorMessage` (`utils/errors.js`) is the single place an axios error becomes
  user-facing text: no `response` means a network/timeout message, otherwise the
  API's `{ error: { message } }` body, falling back to the caller's default.
- `utils/roles.js` mirrors the API's serialized role values exactly
  (`contributor` / `group_admin`, and `SystemAdmin` for the global role).

## UI conventions

Tailwind utility classes are written inline; there is no CSS-module or
styled-components layer, and `main.css` holds only the Tailwind entry plus a few
globals. Interactive primitives that need real accessibility semantics (dialog,
dropdown) wrap Radix; everything else is hand-rolled in `components/ui`. Icons
are named imports from `lucide-react` — no inline SVG.

`AppLayout` renders one `Sidebar` implementation two ways: docked from the `md`
breakpoint up, and a slide-in drawer below it. Visibility is CSS-driven
(`hidden md:flex` / `md:hidden`) so resizing the window never remounts the nav or
drops its state. Pages render content only — the layout supplies the framed
panel and padding.

## Tests

There are **no automated frontend tests** by decision; verification is manual in
the browser. `npm run lint` (ESLint 10, with the React Hooks and React Refresh
plugins) is the only automated check.
