# External Libraries

Every third-party dependency in the project, what it's used for, and where.

---

# Frontend — `frontend/package.json`

## Runtime dependencies

### `react` ^19.2.7 / `react-dom` ^19.2.7
React 19. Features actually relied on:
- `StrictMode` double-invokes effects in dev — the direct cause of the `bootstrapPromise` hoist in `lib/AuthContext.jsx`.
- `createRoot` from `react-dom/client`.
- Hooks only; **no class components anywhere**.

Hooks used across the app: `useState`, `useEffect`, `useCallback`, `useMemo`, `useContext`,
`createContext`. Notably **not** used: `useReducer`, `useRef`, `useId`, `useTransition`,
`use`, or any React 19-specific API like `useActionState` or `useOptimistic`.

### `react-router-dom` ^7.18.0
Client-side routing.
- `BrowserRouter` in `main.jsx`
- `Routes` / `Route` in `App.jsx`, including **layout routes** (a `<Route>` with an `element` and no `path`) — this is what gives the app two chromes with one route table
- `Outlet` in both layouts
- `Link` (buttons, logos, menu items) and `NavLink` (nav rows — its `isActive` render-prop drives active styling)
- `Navigate` with `replace` in `ProtectedRoute` / `AdminRoute`
- `useNavigate` in `AuthContext`, forms, and `MemberManager`
- `useParams` in `GroupManagementPage`

### `axios` ^1.18.1
HTTP client, wrapped once in `lib/axios.js`. Used for:
- `axios.create` with `baseURL`, `withCredentials`, `timeout`
- **Request interceptor** — attaches the bearer token
- **Response interceptor** — the 401 → refresh → retry loop
- `error.response` presence as the network-vs-server discriminator in `utils/errors.js`
- `{ params }` for query strings (omitting `undefined` keys automatically)

Chosen over `fetch` essentially for the interceptors — the transparent refresh-and-retry
would be far more intrusive to write by hand.

### `@tanstack/react-query` ^5.101.2
Server-state cache. One `QueryClient` in `main.jsx` with `retry: 1`.
- `useQuery` — `['groups']`, `['group', id]`, `['group', id, 'members']`, `['admin', 'users', search]`, `['admin', 'groups', search]`, `['admin', 'auditLog']`, `['admin', 'deletionCheck', id]`
- `useMutation` — member role/remove, admin group delete, admin user delete
- `useQueryClient` + `invalidateQueries` — the refresh mechanism throughout
- `keepPreviousData` as `placeholderData` — both admin search panels, so typing swaps results in place instead of flashing a spinner
- Automatic **deduplication** — `GroupStats` and `Sidebar` share `['groups']` and produce one request

Note it's used **selectively**: one-shot forms (login, register, profile, password,
create/rename team) use plain `useState`. React Query is reserved for data something else on
screen also reads.

### `tailwindcss` ^4.3.1 + `@tailwindcss/vite` ^4.3.1
Tailwind v4 via the first-party Vite plugin (no PostCSS config, no `tailwind.config.js`).
`main.css` is the single line `@import "tailwindcss";`.

Every style in the app is a utility class in JSX. Patterns in use: arbitrary opacity
(`bg-white/5`, `border-white/10`), responsive prefixes (`md:flex`, `lg:grid-cols-2`),
`data-highlighted:` variants for Radix menu items, and arbitrary values
(`drop-shadow-[0_0_10px_rgba(56,189,248,0.8)]`).

The relevant gotcha, called out in `Sidebar.jsx`: **Tailwind resolves conflicting utilities
by CSS source order, not by class order in the string** — which is why `Sidebar` takes
positioning via a `className` prop instead of hardcoding one.

### `@radix-ui/react-dialog` ^1.1.19
Backs `components/ui/Modal.jsx`. Supplies Escape handling, outside-click dismissal, focus
trapping, scroll lock, `aria-modal`, and portal rendering. `Dialog.Title` is mandatory for
accessibility, hence the `sr-only` fallback title.

### `@radix-ui/react-dropdown-menu` ^2.1.20
Used twice: `UserMenu` (account/admin/logout) and `MemberManager`'s per-member actions menu.
Supplies keyboard navigation, focus management, `Portal` rendering (escaping overflow
clipping), and the `data-highlighted` attribute the styling hooks into. `Item asChild` wraps
react-router `Link`s so menu navigation stays client-side.

### `lucide-react` ^1.25.0
Icon set, imported as named components. In use: `Menu`, `Pencil`, `MoreVertical`, `Bell`,
`ChevronDown`, `CircleUser`, `LayoutDashboard`, `LogOut`, `PanelLeftClose`, `PanelLeftOpen`,
`Plus`, `Shield`, `Ticket`, `User`, `Users`, `Clock`.

Standalone — no shadcn/ui, no icon wrapper component. Sized with Tailwind (`h-4 w-4`) and
occasionally `strokeWidth={1.5}`.

## Dev dependencies

| Package | Version | Role |
|---|---|---|
| `vite` | ^8.1.0 | Dev server + bundler. Config is 8 lines. |
| `@vitejs/plugin-react` | ^6.0.2 | JSX transform + Fast Refresh. The reason `authContext.js` is split from `AuthContext.jsx`. |
| `eslint` | ^10.5.0 | Linting, `npm run lint` |
| `@eslint/js` | ^10.0.1 | Base recommended rules |
| `eslint-plugin-react-hooks` | ^7.1.1 | Rules-of-hooks + exhaustive-deps |
| `eslint-plugin-react-refresh` | ^0.5.3 | Flags Fast-Refresh-breaking exports |
| `globals` | ^17.6.0 | Browser globals for ESLint |
| `@types/react`, `@types/react-dom` | ^19.x | Editor IntelliSense only — **the project is not TypeScript** (`CLAUDE.md` forbids assuming it) |

### Scripts
`dev` (vite), `build` (vite build), `lint` (eslint .), `preview` (vite preview).

**No test script, no test runner, no test files.** Frontend verification is manual.

### Environment
`VITE_API_URL` (e.g. `http://localhost:8080/api/v1`), read via `import.meta.env`.

---

# Backend — `backend/Cargo.toml`

Rust edition **2024**.

## Dependencies

### `actix-web` "4"
Web framework. `CLAUDE.md` mandates Actix patterns, explicitly not Axum.
- `HttpServer` / `App` / `web::scope` / `web::Data`
- `FromRequest` — the three custom extractors in `server/middleware.rs`
- Built-in extractors `web::Json`, `web::Query`, `web::Path`
- `ResponseError` — implemented on `ApiError`, so handlers return `Result<HttpResponse, ApiError>` and errors render themselves
- `middleware::Logger`
- Cookie building (`actix_web::cookie::Cookie`) with `http_only`, `secure`, `same_site`, `max_age`
- `#[actix_web::main]` and `#[actix_web::test]`

### `actix-cors` "0.7"
CORS layer. Configured with an **explicit** origin (never permissive) plus
`supports_credentials()` — required because the refresh cookie needs credentialed requests,
which the CORS spec forbids combining with a wildcard origin.

### `mongodb` "3"
Official async driver.
- `Client`, `Database`, typed `Collection<T>` (so documents deserialize straight into structs)
- `bson::{doc, oid::ObjectId, DateTime, Document, Regex, to_bson}`
- `find_one`, `find`, `insert_one`, `update_one`, `update_many`, `delete_one`, `delete_many`, `count_documents`
- `find_one_and_update` with `ReturnDocument::After` and `.upsert(true)` — the atomic ticket counter
- `IndexModel` / `IndexOptions` — `unique` and `expire_after` (TTL)
- `ErrorKind::Write(WriteFailure::WriteError)` / `ErrorKind::Command` for duplicate-key (11000) detection

**No aggregation pipelines anywhere** — joins are second queries in the service layer
(`enrich_member`, `enrich_ticket`), a tradeoff the code comments justify explicitly.

### `tokio` "1", features `["full"]`
Async runtime under Actix. Directly referenced only in `tests/support/mod.rs`, which builds
one process-wide multi-thread runtime — the comment explains why: the Mongo driver spawns
SDAM monitor tasks onto whichever runtime was current at `Client` construction, so a
per-test runtime would kill them after the first test.

### `serde` "1", features `["derive"]`
Serialization throughout. Attributes in use: `#[serde(rename = "_id")]`,
`skip_serializing_if = "Option::is_none"`, `rename_all = "snake_case"` (on `Role`,
`TicketStatus`, `TicketPriority`, `AuditAction` — **but not `GlobalRole`**), and
`#[serde(default)]` on the audit log's snapshot name fields for backward compatibility.

Serde also does real validation work: an unknown `status` or `priority` in a request fails
deserialization and yields a 400 before any handler runs.

### `jsonwebtoken` "9"
Access tokens. `encode` / `decode` / `Header` / `EncodingKey` / `DecodingKey` /
`Validation::default()` (HS256, and it validates `exp` automatically).

### `bcrypt` "0.15"
Password hashing at work factor **12** (explicit, not `DEFAULT_COST`).

### `sha2` "0.10"
SHA-256 for refresh-token hashing. A fast hash is correct here — the token already has 256
bits of entropy, so unlike a password there's nothing to brute-force.

### `rand` "0.9"
CSPRNG for refresh tokens: `rand::rng().fill(&mut [0u8; 32])`. Note the 0.9 API (`rng()`,
not the older `thread_rng()`).

### `chrono` "0.4", features `["serde"]`
`DateTime<Utc>` in every response type (serializing to RFC3339), and `Duration` arithmetic
for token expiries. Converted to/from `bson::DateTime` at the persistence boundary.

### `uuid` "1", features `["v4", "serde"]`
**Declared but not used.** No `use uuid` anywhere in `src/` — every id is a Mongo `ObjectId`.
Leftover from earlier design.

### `dotenvy` "0.15"
Loads `.env` in `Config::from_env` and in the test harness. Failure is ignored (`.ok()`) —
in production the vars come from the real environment.

### `futures` "0.3"
- `TryStreamExt::try_collect` — draining Mongo cursors into `Vec`
- `future::LocalBoxFuture` — the return type of the custom extractors' `from_request`

## Dev dependencies

### `serde_json` "1"
Test-only: building request bodies and asserting on response JSON. Also used in the inline
serialization tests in `ticket/models.rs`.

---

## Notable absences

Worth knowing, because their absence shapes the code:

**Frontend**
- No form library (React Hook Form, Formik) — every form is hand-rolled `useState`
- No validation library (Zod, Yup) — `utils/validators.js` is an empty file
- No state manager (Redux, Zustand) — React Query + one small auth context
- No component library (shadcn/ui, MUI) — bare Radix primitives + hand-written `ui/` components
- No test tooling at all
- No date library (date-fns, dayjs) — native `toLocaleDateString`

**Backend**
- No `validator` crate — validation is hand-written functions in each `handlers.rs`
- No `tracing` — just Actix's `Logger`
- No `sqlx`/ORM — the Mongo driver is used directly
- No `mockall` or test doubles — every test runs against a live MongoDB
- No `anyhow`/`thiserror` — error enums and `Display` impls are written by hand
- No Gemini/AI SDK or HTTP client (`reqwest`) — **the AI feature `CLAUDE.md` calls core has no dependency, let alone an implementation**
