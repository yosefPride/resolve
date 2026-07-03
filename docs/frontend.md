# Frontend Architecture

---

# Stack

- React
- JavaScript
- Vite
- TailwindCSS

---

# Core Concept: Group-Centric UI

All UI is scoped to an ACTIVE GROUP.

- No global system view for regular users
- All data shown belongs to selected group

---

# AI UI Rule

AI features must not exist as a standalone page.

All AI interactions must be embedded inside:

- Ticket Detail page (primary)
- Group Dashboard (reports only)

No separate AI navigation tab or page is allowed.

---

# Application States

## 1. Unauthenticated

- Login
- Register

## 2. Authenticated (no active group)

- Group selection page
- Create group
- Join group

## 3. Active Session

- Full application access
- Logout returns to state 1 (Unauthenticated)

---

# Project Structure

src/

    assets/
    components/
        common/
        layout/
        ui/             (design-system primitives: cards, tables, form wrappers)
    features/
        auth/
        groups/
        tickets/
        comments/
        users/
        ai/
        dashboard/
    hooks/
    lib/
    pages/
    services/
    utils/
    App.jsx
    main.jsx

---

# Feature Breakdown

## groups/

- Create group
- Switch active group
- Manage members (Group Admin only)

## tickets/

- Create ticket
- View tickets (group-scoped)
- Update status (role-dependent)
- Assign tickets (Group Admin only)

Ticket Detail page includes:

- ticket content
- comments
- status/actions
- AI panel (contextual sidebar or section)

## ai/ (contextual feature only)

AI functionality is NOT a standalone page.

AI is embedded inside existing pages:

- Ticket Detail Page (primary location)
  - summarize ticket
  - analyze ticket
  - suggest fixes
  - explain ticket

- Group Dashboard (secondary)
  - AI reports (Group Admin only)

---

# RBAC Awareness (Frontend)

Frontend must:

- Hide unavailable actions based on role
- Never rely on UI security (backend is authoritative)
- Always assume server will enforce rules
- reflect user role only for UX purposes

---

# API Layer

- Uses JWT only
- Backend determines active group automatically
- No group_id sent from frontend
- Logout calls POST /auth/logout, then clears the client-stored JWT

---

# Pages

Public:

- Login
- Register

## Setup

- Group selection / creation

## Main App

- Dashboard
- Tickets
- Ticket details (includes AI panel)
- Group management
- Account
- Admin (System Admin only)
