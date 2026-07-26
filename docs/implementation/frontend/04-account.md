# Frontend — Account

Covers: `pages/AccountPage.jsx`, `features/account/` (3 components).

The Account page is where a user manages their own identity — no group scope, no admin
scope. It's the smallest complete feature in the app and a good one to read first, because
it shows the field-level error pattern used elsewhere.

---

## `pages/AccountPage.jsx` (21 lines)

Pure composition, no data fetching of its own:

```jsx
const { user } = useAuth();
...
<ProfileSummary user={user} />   // left column, top
<ProfileForm />                  // left column, bottom
<ChangePasswordForm />           // right column
```

A two-column grid at `lg`, stacking below. `user` comes straight from the auth context —
already loaded during boot, so there is nothing to fetch and no loading state to handle.

---

## `features/account/ProfileSummary.jsx` (36 lines)

Read-only identity header.

### `function initials(name)`
```js
(name ?? '').trim().split(/\s+/).slice(0, 2)
  .map(word => word[0]?.toUpperCase() ?? '').join('')
```
First letter of the first two words — "Ada Lovelace" → "AL". Defensive throughout: `?? ''`
for a missing name, `word[0]?.` for an empty segment, `.slice(0, 2)` so a long name doesn't
produce a wall of letters.

### Render
Avatar circle with the initials, name, a `<Badge variant="accent">System Admin</Badge>` when
`isSystemAdmin(user)`, email, and "Member since {formatDate(user?.created_at)}".

The comment notes data comes straight from the auth context, so there's nothing to fetch.
`truncate` + `min-w-0` on the text column keeps a long email from breaking the flex layout.

---

## `features/account/ProfileForm.jsx` (111 lines)

Edits name and/or email via `PATCH /auth/me`. The interesting part is how it mirrors the
backend's conditional password requirement.

### State
`form` seeded from the context user (`{name, email, currentPassword: ''}`), plus `error`
(form-level), `emailError` (field-level), `success`, `isSubmitting`.

### Derived flags
```js
const nameChanged  = form.name.trim()  !== user.name;
const emailChanged = form.email.trim() !== user.email;
const isDirty      = nameChanged || emailChanged;
```
These drive three separate behaviors:
1. **Submit is disabled** unless `isDirty` — no accidental no-op requests.
2. **The "Current password" field only renders when `emailChanged`**, matching the backend rule that email (the login identity) requires re-authentication while a name change does not.
3. **Only changed fields are sent:**
```js
await updateProfile({
  name:             nameChanged  ? form.name.trim()      : undefined,
  email:            emailChanged ? form.email.trim()     : undefined,
  current_password: emailChanged ? form.currentPassword  : undefined,
});
```
`undefined` keys are dropped by axios's JSON serialization, so the backend receives exactly
the fields that changed — which matters because `validate_update_me` rejects a body that
changes nothing.

### `handleChange(event)`
Generic `name`-keyed setter that **also clears `error`, `emailError`, and `success`** on every
keystroke. So a stale "Profile updated." or a previous error never lingers while editing.

### `handleSubmit(event)` — the error split
```js
if (err.response?.data?.error?.code === 'duplicate_email') {
  setEmailError('Another account is already using this email address.');
} else {
  setError(errorMessage(err, 'Failed to update profile.'));
}
```
A taken email is a **field** problem, shown under the Email input; everything else (wrong
password, validation) stays form-level. This is the pattern to point at when explaining
error handling — the backend's stable `error.code` strings are what make it possible, since
branching on message text would be fragile.

On success: `updateUser(updated)` pushes the new user into the auth context (so the sidebar
and user menu show the new name immediately, no refetch), then the form re-seeds from the
server's response and clears the password field.

---

## `features/account/ChangePasswordForm.jsx` (129 lines)

Three fields — current, new, confirm — posting to `POST /auth/me/password`.

### Constants
`MIN_PASSWORD_LENGTH = 8` (mirrors the backend's `validate_change_password`), and
`EMPTY_FORM` so reset-after-success is a single assignment.

### Client-side validation, before any request
```js
const errors = {};
if (form.newPassword.length < MIN_PASSWORD_LENGTH)
  errors.newPassword = `Password must be at least ${MIN_PASSWORD_LENGTH} characters.`;
if (form.confirmPassword !== form.newPassword)
  errors.confirmPassword = 'Passwords do not match.';
if (Object.keys(errors).length > 0) { setFieldErrors(errors); return; }
```
Two distinct purposes, per the comment: the min-length check **mirrors** the backend (saving
a round trip on a certain rejection), and the confirm check is **client-only** — the backend
never sees `confirmPassword` and has no concept of it.

### `fieldErrors` as an object
Unlike `ProfileForm`'s single `emailError` string, this uses `{[fieldName]: message}`,
because two fields can fail simultaneously. Cleared on every keystroke via `handleChange`.

### Server-error split
```js
if (err.response?.data?.error?.code === 'invalid_credentials') {
  setFieldErrors({ currentPassword: 'Current password is incorrect.' });
} else { setError(errorMessage(err, 'Failed to change password.')); }
```
Same pattern as `ProfileForm`. Worth noting the **rewrite**: the backend's
`invalid_credentials` message is "invalid email or password" — login-oriented and confusing
here — so the component substitutes its own field-appropriate text.

### Success
Resets to `EMPTY_FORM` and shows *"Password changed. Other devices have been signed out."* —
accurately describing `revoke_all_for_user_except`, which revokes every other refresh token
while sparing this session.

### Accessibility details
`autoComplete="current-password"` on the first field and `"new-password"` on the other two,
so password managers offer the right suggestion and don't try to save the old one.

---

## Patterns this feature establishes

Both forms share a shape reused across the app:

1. **Local `useState`, not React Query** — one-shot mutations that nothing else caches.
2. **Clear all messages in `handleChange`** — no stale success/error text while editing.
3. **Field-level vs form-level errors**, chosen by inspecting `error.code`.
4. **`isSubmitting`** disables submit and swaps the label ("Saving…", "Changing…").
5. **`try/catch/finally`** — `finally` always clears `isSubmitting`, even on failure.
6. **Shared `Button` / `Input` primitives**, with layout via `className`.

The one structural difference: `ProfileForm` writes back to global state (`updateUser`)
because the name is displayed in the chrome; `ChangePasswordForm` has nothing to propagate.
