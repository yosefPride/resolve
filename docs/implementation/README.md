# Implementation docs

Two doc sets, deliberately kept apart:

- **[`docs/specification/`](../specification/)** — what the system is *supposed*
  to be. Architecture, API contract, database design, RBAC rules, AI integration.
  Written before and alongside the build; it changes when a decision changes.
- **`docs/implementation/`** (here) — what the code *actually does* right now,
  written from the source. It changes when the code changes.

When the two disagree, the code wins as a description of reality — but the
disagreement is a fact worth recording rather than papering over, which is what
`deviations.md` is for. Neither set is a build diary: there are no per-stage
files, because git history already covers "when", and stage docs rot fastest.

## Which file answers which question

| I want to know… | Read |
| --- | --- |
| How do I run this? What's the tech stack? | [root `README.md`](../../README.md) |
| How does a request get authenticated and authorized? Where does feature X's code live? | [`backend-flow.md`](backend-flow.md) |
| How does the session/token flow work in the browser? Where do I add a page, a hook, a UI primitive? | [`frontend-flow.md`](frontend-flow.md) |
| What fields does collection X have? What's indexed, and why? What happens on delete? | [`data-model.md`](data-model.md) |
| Why doesn't the code match the spec here? | [`deviations.md`](deviations.md) |
| What's the exact request/response shape of an endpoint? | [`../specification/api.md`](../specification/api.md) |
| What are the RBAC rules meant to be? | [`../specification/rbac.md`](../specification/rbac.md) |

## Reading order

New to the codebase: root `README.md` → `backend-flow.md` → `data-model.md` →
`frontend-flow.md`. The backend is the source of truth for behavior, so it makes
more sense before the client that consumes it.

Changing something: check [`deviations.md`](deviations.md) first — the thing that
looks wrong may be a recorded decision rather than a bug.

## Keeping these current

These docs describe code, so they go stale the same way comments do. Practical
rules:

- Change how auth, RBAC, module layout, or error handling works → update
  `backend-flow.md` in the same change.
- Add or reshape a collection, index, or cascade → update `data-model.md`.
  It is written to be checkable against `backend/src/*/models.rs` and
  `backend/src/db.rs`.
- Add a route, layout, or data-layer convention → update `frontend-flow.md`.
- Knowingly diverge from `docs/specification/` → add an entry to
  [`deviations.md`](deviations.md) rather than quietly editing the spec to match.
  When a divergence is resolved — the code changed, or the spec was updated —
  delete the entry.

Prefer explaining *why* over restating *what*: the file listing is discoverable
from the repo, the reasoning is not.
