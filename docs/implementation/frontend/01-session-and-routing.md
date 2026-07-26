# Frontend — Session, Routing & Auth Forms

Covers: `main.jsx`, `App.jsx`, `lib/axios.js`, `lib/authContext.js`, `lib/AuthContext.jsx`,
`lib/ProtectedRoute.jsx`, `lib/AdminRoute.jsx`, `hooks/useAuth.js`,
`services/auth.service.js`, `features/auth/`.

---

## `main.jsx` (25 lines)

The provider stack, outermost to innermost:

```jsx
<StrictMode>
  <QueryClientProvider client={queryClient}>
    <BrowserRouter>
      <AuthProvider>
        <App />
```

### `const queryClient = new QueryClient({ defaultOptions: { queries: { retry: 1 } } })`
React Query defaults to 3 retries with exponential backoff, which makes a failing request
take several seconds to surface an error state. `retry: 1` makes failures visible quickly —
the comment ties this to manual testing, which is how this project is verified.

### Ordering constraints
`BrowserRouter` must wrap `AuthProvider`, because `AuthProvider` calls `useNavigate()` (for
the logout redirect and the unauthorized handler). `QueryClientProvider` sits outside both so
any component can query.

`StrictMode` is on, which double-invokes effects in development — the direct cause of the
`bootstrapPromise` design below.

---

## `lib/axios.js` (85 lines)

The single axios instance every service imports. Four concerns live here.

### Instance config
```js
axios.create({
  baseURL: import.meta.env.VITE_API_URL,   // e.g. http://localhost:8080/api/v1
  withCredentials: true,
  timeout: 15000,
})
```
- `withCredentials: true` is mandatory — without it the httpOnly refresh cookie is neither sent nor stored cross-origin. It's also why the backend must name an explicit CORS origin instead of `*`.
- `timeout: 15000` — the comment explains the failure it prevents: a silently dropped network (Wi-Fi off, blackholed packets) leaves a request pending indefinitely, so callers hang on "Loading…" forever. A timeout rejects with **no `error.response`**, which is the same shape every call site already handles via `utils/errors.js`.

### `let accessToken = null` + `setAccessToken(token)`
Module-scoped variable. **Not React state, not `localStorage`.** The refresh token is
deliberately httpOnly to keep it out of JS reach; storing the access token somewhere
JS-readable and persistent would give back what that was protecting. Cost: a full page
reload loses the token — which is exactly what the boot refresh exists to recover.

### Request interceptor
Attaches `Authorization: Bearer ${accessToken}` when a token is set. Unconditional otherwise
— unauthenticated calls (login, register, refresh) simply go out without it.

### `setUnauthorizedHandler(handler)`
Called once by `AuthProvider` at mount. This module has no React or router access of its
own, so this is how it hands control back when a refresh ultimately fails mid-session.

### `refreshAccessToken()` and `let refreshPromise = null`
The deduplication that makes concurrent 401s safe:

```js
if (!refreshPromise) {
  refreshPromise = api.post('/auth/refresh')
    .then(res => { setAccessToken(res.data.jwt); return res.data.jwt; })
    .finally(() => { refreshPromise = null; });
}
return refreshPromise;
```

Refresh tokens are **single-use**. If three requests 401 simultaneously and each fired its
own refresh, the first would rotate the token and the other two would present an
already-revoked one — turning a recoverable expiry into a forced logout. Sharing one
in-flight promise prevents that. `.finally` clears it so the *next* expiry starts fresh.

### Response interceptor
```js
const shouldAttemptRefresh =
  error.response?.status === 401 &&
  originalRequest &&
  !originalRequest._retry &&
  !NO_RETRY_PATHS.includes(originalRequest.url);
```
Four guards, each load-bearing:
- `status === 401` — only auth failures.
- `originalRequest` exists — a request that never got built can't be replayed.
- `!_retry` — the flag set below, preventing infinite retry loops.
- `NO_RETRY_PATHS = ['/auth/login', '/auth/register', '/auth/refresh']` — a 401 from *login* means wrong credentials, not an expired token; refreshing would be nonsense and would mask the real error.

On a qualifying 401: set `_retry = true`, `await refreshAccessToken()`, overwrite the
original request's `Authorization` header, and re-issue it via `api(originalRequest)`.
If the refresh throws: `setAccessToken(null)`, `unauthorizedHandler?.()`, and reject.

Note the `?.` — if the handler hasn't been registered yet, this degrades to a plain rejection
rather than crashing.

---

## `lib/authContext.js` (3 lines)

```js
export const AuthContext = createContext(null);
```

Separate from `AuthContext.jsx` for **React Fast Refresh**: a module exporting both a
component and a non-component value breaks HMR boundaries. Keeping the bare context here
lets `hooks/useAuth.js` import it without pulling in the provider. Not redundancy.

---

## `lib/AuthContext.jsx` (95 lines)

### `let bootstrapPromise = null` + `function bootstrapSession()` (module scope)

```js
bootstrapPromise = (async () => {
  const { jwt } = await authService.refresh();
  setAccessToken(jwt);
  return authService.me();
})();
```

Module-scoped **specifically because of StrictMode**. The dev-only double effect invocation
would otherwise fire two `POST /auth/refresh` calls; since refresh tokens are single-use, the
second would 401 and flip a valid session to logged-out. Hoisting the promise out of the
component makes the second invocation reuse the first call.

It is never reset, so this runs exactly once per page load — which is correct, since a full
reload creates a new module instance anyway.

### `function AuthProvider({ children })`

**State:** `user` (object or `null`), `status` (`'loading'` → `'authenticated'` | `'unauthenticated'`).

**Effect 1 — register the unauthorized handler** (deps `[navigate]`):
```js
setUnauthorizedHandler(() => {
  setUser(null); setStatus('unauthenticated'); navigate('/login');
});
```
This closes the loop with `lib/axios.js`.

**Effect 2 — bootstrap** (deps `[]`): calls `bootstrapSession()`, then sets
`user`/`status: 'authenticated'` on success, or clears the token and sets `'unauthenticated'`
on failure. Uses a `cancelled` flag in the cleanup so a resolution after unmount doesn't
set state.

**Callbacks** (all `useCallback`, so consumers don't re-render needlessly):

| Function | Behavior |
|---|---|
| `register(input)` | `authService.register` → `setAccessToken(jwt)` → set user → `'authenticated'` |
| `login(input)` | Same shape via `authService.login` |
| `updateUser(updated)` | Replaces the cached user after a profile edit, so the sidebar/menu reflect a new name without a refetch |
| `logout()` | `authService.logout()` → clear token → clear user → `'unauthenticated'` → `navigate('/login')` |

**The gate:**
```js
if (status === 'loading') return <Spinner />;
```
Nothing below this renders until the session question is settled. This is what stops a
logged-in user from briefly seeing the login page after a hard refresh — a race that would
otherwise be guaranteed, since `ProtectedRoute` would evaluate before the boot refresh
resolved.

Context value: `{ user, status, login, register, logout, updateUser }`.

---

## `hooks/useAuth.js` (10 lines)

`useContext(AuthContext)`, throwing `'useAuth must be used within an AuthProvider'` if the
context is `null`. Standard guard — turns a confusing "cannot read property of null" into a
clear message.

---

## `App.jsx` (53 lines)

The full route table. Two **layout routes** (parent routes with an `element` and no `path`,
rendering an `<Outlet />`):

```jsx
<Routes>
  <Route element={<MarketingLayout />}>
    <Route path='/'         element={<LandingPage />} />
    <Route path='/register' element={<RegisterPage />} />
    <Route path='/login'    element={<LoginPage />} />
    <Route path='*'         element={<NotFoundPage />} />
  </Route>

  <Route element={<ProtectedRoute><AppLayout /></ProtectedRoute>}>
    <Route path='/dashboard'  element={<DashboardPage />} />
    <Route path='/account'    element={<AccountPage />} />
    <Route path='/groups/:id' element={<GroupManagementPage />} />
    <Route path='/admin'      element={<AdminRoute><AdminPage /></AdminRoute>} />
  </Route>
</Routes>
```

Three decisions, all explained in the file's comment:

1. **The auth gate wraps the layout, not each page.** One `ProtectedRoute` covers all four authenticated routes.
2. **`AppLayout` is one instance across all authenticated routes**, so the sidebar never remounts when navigating between them — preserving its collapsed/expanded state and its `['groups']` query.
3. **The `*` catch-all sits in the marketing group**, so an unmatched path gets marketing chrome and renders whether or not you're signed in.

`AdminRoute` stays on the `/admin` leaf because it's an *extra* role check layered on top of
the auth gate.

**Missing:** no `/tickets` route, despite `Header.jsx` and `Sidebar.jsx` both linking to it.
Clicking "Issues" hits the catch-all. See [`../deviations.md`](../deviations.md).

---

## `lib/ProtectedRoute.jsx` (12 lines)

```jsx
if (status === 'unauthenticated') return <Navigate to="/login" replace />;
return children;
```
Only checks `'unauthenticated'` — `'loading'` never reaches here, because `AuthProvider`
renders a spinner instead of its children in that state. `replace` keeps the guarded URL out
of history, so Back doesn't bounce.

## `lib/AdminRoute.jsx` (17 lines)

Composes: `<ProtectedRoute>{isSystemAdmin(user) ? children : <Navigate to="/dashboard" replace />}</ProtectedRoute>`.

An unauthenticated user → `/login` (inner guard); an authenticated non-admin → `/dashboard`.
The comment states plainly that the backend enforces every admin action via
`SystemAdminUser` and this guard is UI-only.

---

## `services/auth.service.js` (33 lines)

Seven functions, each `api.<verb>(...).then(res => res.data)`:

| Function | Call | Notes |
|---|---|---|
| `register({email, password, name})` | `POST /auth/register` | Returns `{user, jwt}` |
| `login({email, password})` | `POST /auth/login` | Returns `{user, jwt}` |
| `logout()` | `POST /auth/logout` | Server clears the cookie |
| `refresh()` | `POST /auth/refresh` | Returns `{jwt}` |
| `me()` | `GET /auth/me` | Returns the user |
| `updateProfile({name, email, current_password})` | `PATCH /auth/me` | `current_password` omitted for name-only edits |
| `changePassword({current_password, new_password})` | `POST /auth/me/password` | 200 with no body |

Note the **snake_case parameter names** — these are passed straight through as the JSON body,
so they match the Rust structs rather than JS convention. That's why call sites write
`current_password` rather than `currentPassword`.

---

## `features/auth/LoginForm.jsx` (65) & `RegisterForm.jsx` (76)

Structurally identical; `RegisterForm` just adds a `name` field.

**State:** `form` (a single object), `error`, `isSubmitting`.

### `handleChange(event)`
One handler for all fields, keyed by `event.target.name`:
```js
setForm(prev => ({ ...prev, [name]: value }));
```

### `handleSubmit(event)`
`preventDefault` → clear error → `setIsSubmitting(true)` → `await login(form)` (from
`useAuth()`, **not** the service directly — the context owns session state) →
`navigate('/dashboard')`. On failure: `setError(errorMessage(err, 'Invalid email or
password.'))`. `finally` clears `isSubmitting`.

No client-side validation beyond the native `required` and `type="email"` attributes — the
backend's `validate_register` / `validate_login` are the real check, and their messages
surface through `errorMessage`. (`utils/validators.js` exists but is empty.)

Both use the shared `Button` and `Input` primitives and show a busy label
("Logging in…" / "Creating account…") while submitting.
