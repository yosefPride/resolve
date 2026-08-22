# Deviations from the specification

Where the code and [`docs/specification/`](../specification/) disagree, and why.

This file exists so the spec can stay a statement of intent without quietly
becoming false. Every entry names the spec file making the claim, what the code
actually does, and what should happen about it. An entry is removed when it is
resolved — either the code changed, or the spec was updated to match.

Verified against the code on 2026-08-22.

**Legend for the "Resolution" line:**
`spec` = the spec should be updated; `code` = the code should change;
`accepted` = a deliberate, permanent divergence that stays recorded here.

---

## 1. Built but not specified

### 1.1 Comment reactions are entirely absent from the spec

Emoji reactions on comments are fully implemented — a `comment_reactions`
collection, `PUT` and `DELETE /groups/{id}/tickets/{ticket_id}/comments/{comment_id}/reactions`,
and a `reactions` array on every comment response. The word "reaction" does not
appear anywhere in `docs/specification/`: not in `api.md`'s Comment Endpoints,
not in `database.md`'s collection list, not in `frontend.md`'s feature list.

**Resolution:** spec. Needs an endpoint pair in `api.md`, a collection in
`database.md`, and the one-per-user-per-comment replace rule written down
somewhere (see [`data-model.md`](data-model.md#comment_reactions)).

### 1.2 AI chat conversations are absent from the spec

Seven implemented routes — conversation create/list/delete plus message
send/list — are undocumented. `api.md` documents only `summarize` and `analyze`
under AI Endpoints, and `ai-integration.md` describes AI purely as one-shot
ticket analysis. Neither mentions conversations, and `database.md` has neither
`ai_conversations` nor `ai_chat_messages`.

This also carries the system's strictest ownership rule, which is written down
nowhere: a conversation is private to its creator, and **no other member — not
even a Group Admin — can list, read, or delete it**. That is deliberately unlike
comments, which are group-visible. A rule that strong should not live only in a
code comment.

**Resolution:** spec. Document the endpoints, both collections, and the
ownership rule.

### 1.3 Rate limiting exists but no spec mentions it

AI chat is capped at 10 user messages per hour per user, returning `429` with
`ApiError::RateLimited`. `api.md`'s Status Codes list does not include `429` at
all, and no spec file mentions rate limiting.

**Resolution:** spec. Add `429` to the status code list and the limit to the AI
docs.

### 1.4 `PATCH /groups/:id` (rename a group) is undocumented

The route exists and is Group-Admin-gated. `api.md`'s Group Endpoints section
lists create, list, get, delete, and the member operations — but no rename.

**Resolution:** spec.

### 1.5 Four backend modules and two frontend feature folders are missing from the structure lists

`backend.md`'s module tree omits `activity/`, `link/`, `reaction/`,
`reference/`, and `db.rs`. `frontend.md`'s `features/` tree omits `activity/`
and `links/`.

**Resolution:** spec. Purely a stale listing — the modules follow the documented
layering exactly.

---

## 2. Specified but not built

### 2.1 `GET /admin/analytics` does not exist

`api.md` documents it ("System-level metrics (aggregated only)"), and it is
load-bearing for three other claims: `architecture.md` lists "aggregated
analytics" as a System Admin exception to group isolation, `backend.md` lists
"view system analytics (aggregated only)" as a capability, and
`ai-integration.md` has a whole "System Admin Analytics Rule" section about
AI-generated summaries of group-level statistics.

None of it is implemented. `/admin` has users, groups, group delete, audit log,
deletion-check, delete, promote, and demote — no analytics route.

**Resolution:** undecided — either build it or cut it from four spec files. It
should not stay documented-but-absent, since a reader currently has no way to
tell it is aspirational.

### 2.2 AI bug clustering is not implemented

`ai-integration.md` lists "Bug grouping (light clustering within a group)" as a
capability and devotes a "Clustering Definition" section to it; `architecture.md`
lists it too. Nothing in `ai/` clusters anything.

Note the near-miss: **bug *classification* is implemented** — `analyze` returns
`classification` alongside `severity_prediction` and `suggested_fix`. Clustering
(grouping similar tickets to each other) is the part that does not exist.

**Resolution:** undecided. Cut it or build it, but the two similar-sounding
features should stop being conflated.

### 2.3 Group-level AI reports do not exist

`architecture.md` lists "Group-level AI reports", `backend.md` lists "Group
reports (async recommended)", and `frontend.md` places "AI reports (Group Admin
only)" on the Group Dashboard as AI's secondary location. There is no such
endpoint, and the dashboard has no AI surface — AI appears only on the ticket
detail page.

**Resolution:** undecided, same as 2.1 and 2.2. These three are one feature
family: system/group-level AI aggregation was designed and never built.

### 2.4 There is no "My groups" page

`frontend.md` lists "My groups — list the groups you belong to, and create one"
under Setup. No such page or route exists. In practice the group list is reached
through the sidebar and the Tickets page's team dropdown, and the create-a-group
prompt lives in the Tickets page's empty state. `/groups/:id` (group management)
exists; `/groups` does not.

**Resolution:** spec. The behavior the page was meant to provide is covered
elsewhere; the page itself was dropped.

---

## 3. Built differently than described

### 3.1 `GroupScoped` does not carry the caller's role

Both `backend.md` ("Handlers receive the resolved `{ user_id, group_id, role }`")
and `rbac.md` ("resolves the caller's current role in one lookup") describe the
extractor as resolving and handing over the role. It does not: `GroupScoped` is
`{ user_id, group_id }` only. It calls `require_member` to verify membership and
discards the returned role; services re-derive it where a role check is needed.

The security posture is unchanged — membership is still checked per request, and
role checks still run at the service layer — but the field the spec promises is
not there, so anyone writing a handler against the documented shape will not find
it.

**Resolution:** spec, most likely. Carrying the role would save a lookup on
admin-gated routes, so changing the code instead is defensible; either way the
two must be reconciled.

### 3.2 The access token lives in a module variable, not React context

`frontend.md` says the JWT is "held in memory only (React context)". It is held
in a module-scoped variable inside `lib/axios.js`, not in context at all —
`AuthContext` holds the *user*, and calls `setAccessToken` to hand the token to
the axios layer.

The security property the spec cares about is intact and arguably better: the
token is in memory only, never in `localStorage`, and unreachable from React
state. But someone auditing "where is the token" will look in the wrong file.

**Resolution:** spec.

### 3.3 `database.md`'s collection fields are behind the code

Four collections are missing fields that exist and matter:

| Collection | Missing from `database.md` |
| --- | --- |
| `tickets` | `content_updated_at` — the AI cache fingerprint |
| `comments` | `parent_comment_id`, `is_deleted` — threading and tombstoning |
| `ai_ticket_insights` | `summary_source_updated_at`, `analysis_source_updated_at` |
| `ai_conversations`, `ai_chat_messages` | the collections themselves (see 1.2) |

The `comments` gap is the odd one: `api.md` documents threading and tombstoning
in full, so the two spec files disagree with each other, not just with the code.

The recommended indexes are also stale in two places — `database.md` suggests
separate `ticket_id` and `group_id` indexes on both `comments` and
`ai_ticket_insights`, where the code builds a compound `{group_id, ticket_id}`
(unique, on insights) plus `{parent_comment_id}` on comments.

**Resolution:** spec. [`data-model.md`](data-model.md) already records the
as-built shape.

---

## 4. Absolute rules that are narrower than stated

These are not bugs. They are places where a spec sentence is written as
universal, the code is correct, and a reader taking the sentence literally would
be misled.

### 4.1 "EVERY database query MUST include group_id filter. No exceptions."

`backend.md` and `database.md` both state this unconditionally. Several
collections are not group-scoped and cannot be: `users`, `refresh_tokens` (session
data tied to a user), `admin_audit_log` (system data, explicitly called out
elsewhere in `database.md` as not tenant data), and `counters` (keyed *by* group
id as `_id`). Within AI, messages are listed by `conversation_id`, and the
rate-limit count filters on role, user, and time.

The rule the code actually follows: **every query against group-scoped business
data filters on `group_id`.** That holds without exception, and it is the rule
that matters. The universal phrasing just makes the correct code look like a
violation.

**Resolution:** spec — narrow the wording to group-scoped business data.

### 4.2 "AI never writes to database"

Stated in `architecture.md`, `backend.md`, and `ai-integration.md`. The AI module
does write: `ai_ticket_insights`, `ai_conversations`, and `ai_chat_messages`.
`ai-integration.md` even contradicts itself four sections later — "Result is
stored (cached) and returned to frontend".

The real rule, which the code follows exactly: **AI never writes domain state.**
It reads tickets and never modifies them, or comments, or membership, or roles.
It only writes its own AI-owned collections.

**Resolution:** spec — restate as "never writes domain state".

### 4.3 "Triggered by user actions or ticket creation"

`ai-integration.md` says AI runs on user action *or* ticket creation. Creating a
ticket fires no AI call; every AI call in the system originates from an explicit
user action on the ticket detail page. This is consistent with AI being advisory
and with the cost-control posture elsewhere in the same file.

**Resolution:** spec.

---

## 5. Outside the spec, but worth recording here

### 5.1 The UI says "Team", everything else says "group"

A deliberate, UI-only rename. Backend, API, database, and every prop, hook, query
key, and route still use `group`/`groupId`; only user-visible strings changed.
Not a bug, and not planned to be reconciled — the spec's vocabulary is correct
for the layers it describes.

**Resolution:** accepted.

### 5.2 There are no automated tests

No `tests/` directory, no `#[cfg(test)]` modules; frontend testing is manual by
decision. Several backend comments still describe test-only affordances
(`jwt::issue_token_with_exp`, `AiService::with_provider`, free functions in
`ai/service.rs` kept unit-testable) whose tests no longer exist. No spec file
claims tests exist, so this is not strictly a deviation — but anyone reading
those comments will go looking for a suite that is not there.

**Resolution:** undecided.
