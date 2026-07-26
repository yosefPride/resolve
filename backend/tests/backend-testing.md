# Backend Testing — How It Works

A plain-language guide to the backend test suite: what it covers, how it's
organized, and how a single test actually runs. Written to be read aloud or
explained in a walkthrough.

---

## The one-sentence version

The backend has ~126 automated tests that run against a **real MongoDB**, and
they are organized to mirror the backend's layered architecture — so each
feature is checked at the database level, the business-logic level, and the
full HTTP level.

Why it matters: the whole system's job is security — group isolation and RBAC —
and the project rule is *"the backend is the source of truth, all security is
enforced server-side."* Testing the backend thoroughly is therefore where the
real assurance comes from.

---

## The mental model: three layers

The backend is built in layers, and a request flows down through them:

```
HTTP request
    │
    ▼
Handler      ← receives the request, validates input
    │
    ▼
Service      ← business logic + permission (RBAC) rules
    │
    ▼
Repository   ← the only layer that talks to MongoDB
    │
    ▼
MongoDB
```

The tests mirror those layers exactly. Every feature is tested at the level(s)
that matter for it:

| Layer          | What it checks                                   | Runs against    |
| -------------- | ------------------------------------------------ | --------------- |
| **Repository** | Data reads/writes + DB constraints (unique keys) | Real MongoDB    |
| **Service**    | Business rules + who-is-allowed-to-do-what       | Real MongoDB    |
| **API**        | The whole request path through real routing      | Real MongoDB    |

The API-level tests are the most complete: they send a real HTTP request through
the actual Actix router, so a single test exercises **authentication → group
membership resolution → RBAC → handler → database → JSON response** all at once.

---

## The test files, by layer

Total: ~126 tests across 10 files.

**Repository layer** (data + database constraints):
- `tests/api_tests.rs` — the user repository (create, find, delete, duplicate-email constraint)
- `tests/group_repository_tests.rs` — groups & memberships (incl. the unique-membership index)
- `tests/admin_repository_tests.rs` — the audit-log store

**Service layer** (business logic + permissions):
- `tests/group_service_tests.rs` — group rules (who can rename/delete/add/remove; the "at least one Group Admin" invariant)
- `tests/rbac_service_tests.rs` — the shared permission helpers (member / group-admin / system-admin / owner checks)
- `tests/admin_service_tests.rs` — System-Admin user/group management + the succession logic

**API layer** (full HTTP through the real routes):
- `tests/auth_tests.rs` — register/login/me/refresh/logout + the account endpoints (this file spans both the service and HTTP levels)
- `tests/group_api_tests.rs` — group endpoints end to end
- `tests/rbac_api_tests.rs` — RBAC enforced at the real endpoints (401 vs 403)
- `tests/admin_api_tests.rs` — admin endpoints end to end, incl. the full deletion/succession flow

---

## How a single test runs (the flow)

Every test follows the same three beats — **Arrange → Act → Assert**:

1. **Arrange** — `setup()` connects to the test database (`resolve_test`),
   starts from a clean slate, and creates whatever the test needs (a user, a
   group, a membership).
2. **Act** — the test does *one* thing: calls a repository method, a service
   method, or sends one HTTP request.
3. **Assert** — it checks the result is exactly what the rule requires (the
   right value, or the right rejection like `403` or `409`).

For an **API test**, the "Act" step is a real HTTP request, so the flow inside
that one step is the full request lifecycle:

```
TestRequest (e.g. PATCH /auth/me with a JWT)
    │
    ▼
JWT validated              → no/expired token? 401, stop
    │
    ▼
Group membership resolved  → not a member? 403, stop      (group-scoped routes)
    │
    ▼
RBAC check                 → wrong role? 403, stop
    │
    ▼
Handler runs → Service → Repository → MongoDB
    │
    ▼
JSON response  → the test asserts on the status + body
```

That's why the API tests are the strongest evidence: one green test proves the
entire chain works together, not just one piece in isolation.

---

## Why real MongoDB (not mocks)

These are **integration tests**, on purpose. The security guarantees — "a
non-member cannot read another group's data", "the unique-email index rejects
duplicates" — only mean something when checked against the actual database and
the actual routing. A mock would just be re-stating our own assumptions.

The trade-off: because they share one live database, they **run serially**
(`--test-threads=1`) so they don't interfere with each other.

---

## Run them

```bash
# all tests (serially — required, they share one database)
cargo test -- --test-threads=1

# one file
cargo test --test auth_tests -- --test-threads=1

# one test by name
cargo test test_change_password_revokes_other_sessions -- --test-threads=1
```

They need a `MONGO_URI` in the environment (a `.env` is picked up
automatically).

---

## The headline tests (be ready to walk through these)

Each one demonstrates a **design decision**, not just code:

1. **Email change requires the current password** — `tests/auth_tests.rs:609`
   A name change is free; changing the email (the login identity) demands the
   current password. Tries it three ways: no password → 400, wrong → 401,
   correct → 200, then confirms login works with the new email.

2. **Changing the password logs out other devices** — `tests/auth_tests.rs:820`
   Logs in on two simulated devices, changes the password on one, and checks:
   old password dead, new one works, the *other* device is signed out, but the
   device that made the change stays logged in.

3. **A revoked member loses access on the very next request** — `tests/rbac_api_tests.rs:190`
   Proves the key RBAC design point: the JWT carries no role — membership and
   role are looked up fresh on every request. Remove a member, and their next
   request is a 403 immediately.

4. **Group-Admin succession is safe and atomic** — `tests/admin_api_tests.rs:96`
   (with `tests/admin_service_tests.rs:427` as backup)
   Deleting a user who is a group's sole Group Admin first returns the blocked
   group and its eligible successors, then promotes the named successor and
   removes the user. The service test proves atomicity: sole admin of two groups
   but only one successor named → the whole deletion is rejected, nothing is
   changed.

Together these cover the entire core platform — auth, sessions, RBAC/isolation,
and the "every group always has an admin" invariant.
```
