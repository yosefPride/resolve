# Frontend — Layout, UI Primitives & Utils

Covers: `components/layout/` (6 files + 1 empty), `components/ui/` (5), `components/marketing/`
(4), `utils/` (3 + 1 empty), `pages/` (the remaining thin ones), `main.css`, `index.html`.

---

## Two layout shells

The app has two distinct chromes, selected by the layout routes in `App.jsx`.

### `components/layout/MarketingLayout.jsx` (17 lines)
For `/`, `/login`, `/register`, and the `*` catch-all. A flex column: `<Header />` +
`<Outlet />` in a `grow` div, with `<Footer />` pinned to the bottom via `min-h-screen`.

### `components/layout/AppLayout.jsx` (60 lines)
For all authenticated routes. Renders `Sidebar` **twice**, from one implementation:

```jsx
<Sidebar className="sticky top-0 hidden md:flex" />          {/* docked, md+ */}
{isDrawerOpen && (
  <>
    <button ... className="fixed inset-0 z-40 ... md:hidden" />   {/* scrim */}
    <Sidebar className="fixed inset-y-0 left-0 z-50 flex md:hidden"
             collapsible={false} onNavigate={closeDrawer} />
  </>
)}
```

Visibility is **CSS-driven** (`hidden md:flex` / `md:hidden`) rather than JS-driven, so
resizing the window never remounts the nav or drops its state.

The drawer variant passes `collapsible={false}` (a 16-wide rail inside a drawer would be
pointless) and `onNavigate={closeDrawer}` so following a link closes it.

Below `md`, a sticky mobile header holds a hamburger and the logo. `<main className="grow">`
holds the `<Outlet />`.

---

## `components/layout/Sidebar.jsx` (321 lines — largest file in the app)

### `SidebarContext`
A local context (`{collapsed, expand, onNavigate}`) so collapsed state reaches nav rows
without prop-drilling through every section.

### Styling constants
`ROW`, `IDLE`, `ACTIVE`, and `rowClasses(collapsed, isActive)` — one function producing every
row's classes, which is why all rows stay visually consistent.

### `NavItem({ to, label, icon, end })`
A `NavLink` whose `onClick` calls **both `expand()` and `onNavigate()`** — so clicking a link
while collapsed both follows it and restores the full sidebar.

### `UserSection({ user, onLogout })`
An inline-expanding account menu (not a floating dropdown, so the sidebar stays one
continuous surface). Contains Account, Admin (only when `user?.global_role === 'SystemAdmin'`),
and Log out.

`handleToggle` has a special case: when collapsed, the row is too narrow for the menu, so a
click **expands the sidebar and opens the menu in one step**.

### `TeamsSection()`
```js
const { data: groups = [], status } = useQuery({ queryKey: ['groups'], queryFn: listGroups });
```
Shares the `['groups']` key with `GroupStats`, so creating/renaming/deleting a team anywhere
refreshes this list through invalidations those pages already run — no extra plumbing.

A `+` button opens a `Modal` with `CreateGroupForm`; `handleCreated` invalidates `['groups']`,
closes the modal, and force-opens the section so the new team is visible.

Collapsed, it renders a single icon button that just re-expands the sidebar — there is no
Teams page to navigate to (the section is a header over a live list).

Handles all four query states: pending, error, empty, populated.

### `Sidebar({ className, collapsible = true, onNavigate })` (default export)

Collapsed state persists to `localStorage` under `'sidebar:collapsed'`, read lazily in the
`useState` initializer. `const collapsed = collapsible && storedCollapsed` — the drawer
ignores the stored value.

The `className` prop carries positioning/display, and the comment explains why that matters:
**Tailwind resolves conflicting utilities by CSS source order, not class order**, so a
`fixed` passed in would not reliably beat a hardcoded `sticky`. Hence no positioning class is
hardcoded in the component.

Contains a deliberately **inert Notifications row** — `aria-disabled="true"`,
`cursor-not-allowed`, muted colors, and a "soon" badge. The comment: no notifications backend
yet, so the row reads as planned rather than broken.

`NAV_LINKS` = Dashboard (`/dashboard`) and Issues (`/tickets`). **`/tickets` has no route** —
its comment says "the route stays /tickets to match the backend", but `App.jsx` never
registers it, so it 404s. See [`../deviations.md`](../deviations.md).

---

## `components/layout/Header.jsx` (113 lines)

The marketing header. Sticky, `backdrop-blur`, `z-50`.

- `Logo({ isAuthenticated })` — links to `/dashboard` or `/`, with a hover glow.
- `NavItem` — pill-styled `NavLink` with an active background.
- Authenticated → desktop nav + `<UserMenu />` + a mobile hamburger. Unauthenticated → "Log in" / "Sign up" buttons.
- Mobile nav is a separate stacked block below the bar, toggled by `isMobileNavOpen`.

Uses the same `NAV_LINKS` (Dashboard, Issues → `/tickets`), so the broken link appears in both chromes.

## `components/layout/UserMenu.jsx` (67 lines)

A Radix `DropdownMenu` behind a circular avatar trigger. Label row shows the user's name with
an animated green "online" dot (a `animate-ping` span layered under a solid one). Items:
Account, Admin (System Admin only), Log out. Uses `DropdownMenu.Item asChild` wrapping a
react-router `Link`, so navigation is client-side rather than a full page load.

## `components/layout/Footer.jsx` (34 lines)

Logo, `© {new Date().getFullYear()} Resolve`, and a `v0.1.0` badge.
`const FOOTER_LINKS = []` — an empty array with a `.map()` over it, structure in place for
links that don't exist yet.

## `components/layout/PageWrapper.jsx`
**Empty file (0 bytes).** Each page hardcodes its own
`mx-auto max-w-* px-4 py-20 sm:px-6 lg:px-8` wrapper instead — the abstraction was planned
and never extracted.

---

## `components/ui/` — the design-system primitives

### `Button.jsx` (45 lines)
`BASE` + `VARIANTS` (`primary`, `ghost`, `danger`, `dangerOutline`) + `SIZES` (`sm`, `md`, `lg`).

Two behaviors worth knowing, both from the comment:
- **Polymorphic**: passing `to` renders a react-router `<Link>` styled as a button; otherwise a `<button>` defaulting to `type="button"` so it never submits a form by accident.
- **Hover lives per-variant**, not in `BASE`: filled variants dim via `opacity`, transparent ones (`ghost`, `dangerOutline`) get a low-opacity background instead — dimming a see-through button reads poorly. `font-weight` is also per-variant so the two never fight over CSS source order.

### `Input.jsx` (10 lines)
One `BASE` string plus `{...props}` passthrough. All variation comes through `className`.

### `Badge.jsx` (21 lines)
Variants `neutral` (default gray), `accent` (sky-tinted, e.g. the System Admin chip),
`outline` (muted border, e.g. the footer version tag). Sizes `sm` / `md`.

### `Modal.jsx` (25 lines)
A thin wrapper over Radix `Dialog` preserving the app's original API
(`{isOpen, onClose, title, children}`). Radix supplies Escape handling, outside-click,
focus trapping, scroll lock, and `aria-modal`.

Two details: `<Dialog.Title>` is **always** rendered (visually hidden via `sr-only` when no
`title` is given) because Radix requires one for accessibility; and
`aria-describedby={undefined}` suppresses Radix's warning about a missing description.

### `Spinner.jsx` (7 lines)
A full-screen centered spinning ring. Used only by `AuthProvider` during boot — everything
else uses inline "Loading…" text.

---

## `components/marketing/` — the landing page

`LandingPage.jsx` composes four sections in order: `Hero` (39), `FeatureGrid` (66),
`WorkflowTimeline` (57), `AudienceCards` (86). All static presentational content — no data
fetching, no props. This is the only place the `marketing/` directory is used.

---

## `utils/`

### `errors.js` (11 lines) — used in nearly every component
```js
export function errorMessage(err, fallback = 'Something went wrong. Please try again.') {
  if (!err.response) return 'Network error — check your connection and try again.';
  return err.response.data?.error?.message || fallback;
}
```
Three cases, per the comment: **no response** (network down, CORS, or the 15s timeout, which
arrives as `ECONNABORTED`) → a network message, since there's no server payload to read;
**response present** → the API's `{error: {message}}` shape; **unexpected body shape** →
the caller's fallback.

The single `!err.response` check covering offline *and* timeout is exactly why
`lib/axios.js` set a timeout in the first place — it produces the shape this already handles.

### `roles.js` (14 lines)
```js
export const GROUP_ROLES = { CONTRIBUTOR: 'contributor', GROUP_ADMIN: 'group_admin' };
export function isGroupAdmin(role)  { return role === GROUP_ROLES.GROUP_ADMIN; }
export function isSystemAdmin(user) { return user?.global_role === 'SystemAdmin'; }
```
The comment ties both conventions to the backend: `Role` carries
`#[serde(rename_all = "snake_case")]`, `GlobalRole` has no rename. Hence snake_case for group
roles and PascalCase for the global one — an asymmetry that only makes sense with the Rust
side in view.

Note the different signatures: `isGroupAdmin` takes a **role string**, `isSystemAdmin` takes
a **user object**. That reflects where each value lives — group role comes from a membership
row, global role from the user.

### `format.js` (28 lines)
`formatDate(iso)` and `formatDateTime(iso)` — both guard `!iso` and `Number.isNaN(getTime())`,
returning `'—'`. Use `toLocaleDateString`/`toLocaleString` with `undefined` as the locale, so
output follows the viewer's browser locale.

### `validators.js`
**Empty file (0 bytes).** Validation is inline in each form plus native HTML attributes.

---

## Styling setup

- **`main.css`** is one line: `@import "tailwindcss";` — Tailwind v4, which needs no config file and no `@tailwind base/components/utilities` triple.
- **`vite.config.js`** registers `@vitejs/plugin-react` and `@tailwindcss/vite` (the v4 first-party plugin, replacing the PostCSS pipeline).
- **`index.html`** sets `<title>Resolve</title>`, the SVG favicon, and an inline `body { background-color: black; }` — inlined so there's no white flash before the CSS bundle loads.
- No CSS modules, no styled-components, no `.css` files beyond `main.css`. Every style is a Tailwind utility class in JSX.
- Dark-only: colors are hardcoded (`bg-black`, `text-white`, `border-white/10`) with no `dark:` variants or theme toggle.

---

## The thin pages

| Page | Lines | Content |
|---|---|---|
| `LandingPage` | 15 | Composes the four marketing sections |
| `LoginPage` | 10 | Heading + `<LoginForm />` |
| `RegisterPage` | 10 | Heading + `<RegisterForm />` |
| `DashboardPage` | 13 | **Placeholder** — "Welcome, {name}" and a logout button |
| `NotFoundPage` | 22 | 404 text + a context-aware button (`/dashboard` if authenticated, else `/`) |

`NotFoundPage`'s comment notes that while the boot refresh is still resolving we don't know
the auth state, so it points at the landing page — reachable either way.
