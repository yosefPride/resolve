![Resolve](/frontend/public/preview.png)

# RESOLVE

A multi-tenant bug tracker built around strict group isolation and two independent
RBAC layers, with AI-assisted ticket workflows. Node.js + MongoDB on the backend,
React 19 + Vite + Tailwind on the frontend.

## What it does

- **Teams (groups) as tenants** — every ticket and comment belongs to exactly one
  group, and no data ever crosses a group boundary. Any user can create a group and
  becomes its Group Admin.
- **Two-layer RBAC** — group-scoped roles (Contributor / Group Admin) govern all
  ticket and comment work; a separate global System Admin role covers system
  metadata only and cannot read ticket data in groups it doesn't belong to. All
  enforcement happens on the backend.
- **Tickets & collaboration** — ticket CRUD with comments, reactions, cross-ticket
  links, and a per-ticket activity log.
- **AI assistance (Gemini)** — advisory features layered on top of tickets. AI never
  writes to the database and always respects group boundaries; the backend runs fine
  without an API key (only the AI endpoints go down).
- **Admin panel** — user and group management for System Admins, including the
  audited succession flow for deleting a group's sole admin.

## Repository layout

| Path | Contents |
| --- | --- |
| `backend/` | Node.js API (plain JavaScript on the built-in `node:http` server). One module per feature (`auth`, `group`, `rbac`, `ticket`, `comment`, `activity`, `link`, `reaction`, `admin`, `ai`, ...), each following the same Handler → Service → Repository layering. |
| `frontend/` | React SPA. Feature folders under `src/features/`, routed pages under `src/pages/`, shared UI in `src/components/`. |
| `docs/specification/` | Design source of truth: architecture, API, database, RBAC, AI integration. |
| `docs/implementation/` | How the system is actually built: backend flow, frontend flow, data model, deviations from spec. |

## Getting started

Prerequisites: Node.js 22 or newer and a MongoDB instance (local or Atlas).

### Backend

```bash
cd backend
cp .env.example .env   # then fill in real values
npm install
npm start
```

The server binds to `0.0.0.0:8080` (or `$PORT`) and serves the API under `/api/v1`.

Environment variables (loaded from `backend/.env`):

| Variable | Required | Purpose |
| --- | --- | --- |
| `MONGO_URI` | yes | MongoDB connection string. |
| `JWT_SECRET` | yes | Signing key for access tokens. |
| `COOKIE_SECURE` | no | Set to `false` for local HTTP development so the browser will store the refresh-token cookie. Defaults to `true` (required in production). |
| `FRONTEND_ORIGIN` | no | Origin allowed by CORS for credentialed requests. Defaults to `http://localhost:5173`. |
| `GEMINI_API_KEY` | no | Enables the AI endpoints. Everything else works without it. |
| `PORT` | no | Port to listen on. Defaults to `8080`. |
| `MONGO_DB` | no | Database name. Defaults to `resolve`. |
| `LOG_LEVEL` | no | `error` (default) or `info` for a per-request access log. |

### Frontend

```bash
cd frontend
cp .env.example .env   # points VITE_API_URL at the backend
npm install
npm run dev
```

Vite serves the app on `http://localhost:5173`, talking to the backend via
`VITE_API_URL` (default `http://localhost:8080/api/v1`).

## Documentation

Start with [`docs/specification/architecture.md`](docs/specification/architecture.md)
for the intended design, then [`docs/implementation/`](docs/implementation/) for how
the code is actually organized and where it deviates from the spec.
