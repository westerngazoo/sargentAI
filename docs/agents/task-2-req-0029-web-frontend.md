# Agent Task — Write Requirement R-0029 (Web Frontend)

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** create `R-0029-web-frontend` from `main`, push to it
**Output file:** `requirements/0029-web-frontend.md`

---

## What you are doing

You are writing a new accepted requirement file for the fitAI project. The owner
has approved adding a web frontend so the product is accessible from a browser,
not just the Flutter mobile app.

---

## Step 1 — Read these files before writing anything

```
requirements/0014-program-diet-from-archetype.md   ← requirement format to match exactly
CLAUDE.md                                           ← engineering constitution
project-specifics.md                                ← project identity and stack
backend/crates/api/src/lib.rs                       ← all existing API routes
backend/crates/api/src/program/mod.rs               ← program routes
backend/crates/api/src/matching/mod.rs              ← matching routes
```

---

## Step 2 — Context

The backend is a plain JSON REST API (Axum, JWT HS256 auth). The Flutter app is
one consumer of it. A web frontend is a second consumer — same API, same JWT.

**Owner's stated goal:** "ease of use, accessible from a browser." Target users:
the same end users on desktop/laptop, plus potentially coaches managing clients.

**Technology decided by owner:**
- Framework: **Next.js** (React, TypeScript) — SSR, file-based routing
- Styling: **Tailwind CSS**
- Auth: same JWT the mobile app uses (stored in httpOnly cookie via Next.js API
  route to avoid XSS exposure of token in localStorage)
- Location: new `/web` directory in the existing monorepo
- **No new backend endpoints for MVP** — consume only what already exists

**Existing API endpoints the web client will use:**
- `POST /auth/register`, `POST /auth/login` — auth
- `GET /auth/me` — whoami
- `GET /profile/me`, `PUT /profile/me` — profile
- `POST /photo-sessions`, `POST /photo-sessions/:id/photos` — photo upload
- `POST /photo-sessions/:id/match` — archetype matching
- `GET /photo-sessions/:id/program-proposals` — proposals
- `POST /programs` — choose program
- `GET /programs/me/current` — active program
- `GET /programs/me` — program history
- `GET /workouts` — workout history

---

## Step 3 — Write the requirement file

Follow the **exact format** of `requirements/0014-program-diet-from-archetype.md`.

**Metadata:**
```
Status: Accepted
Milestone: M3 (fast-track) / cross-cutting
Created: 2026-06-21
Depends on: R-0002 (auth), R-0003 (profile), R-0006 (photo session),
            R-0013 (matching), R-0014 (program+diet)
```

**Statement:** A Next.js web application that gives users browser access to the
core fitAI loop: register/login → profile setup → photo upload → archetype
matching → program proposals → active program view → workout history.

**Acceptance criteria (write 10–12)** covering:
- AC1: Register + login via the existing API; JWT stored in httpOnly cookie
- AC2: Profile setup form (`PUT /profile/me`)
- AC3: Photo upload + match flow (file input → `POST /photo-sessions` →
  `POST /photo-sessions/:id/photos` → `POST /photo-sessions/:id/match`)
- AC4: Proposals screen (top-3 cards, expand, choose)
- AC5: Active program detail page
- AC6: Workout history list
- AC7: Responsive layout (mobile-first, works on 375px–1440px)
- AC8: Auth-gated routes (redirect to login if no valid session)
- AC9: No new backend endpoints — only existing API consumed
- AC10: TypeScript strict mode; `next lint` clean; `tsc --noEmit` clean
- AC11: At minimum smoke tests: render register page, render login page,
  render home page when mocked as authenticated
- AC12: `next build` succeeds in CI (add to GitHub Actions workflow)

**Constraints:**
- No new backend endpoints
- No server-side rendering of user-specific data without auth (cookie-gated)
- Famous-athlete internal names never rendered in the UI (same rule as mobile)
- No billing/gating UI (M7)
- No nutrition log UI (R-0010, deferred)
- No dashboard/trends (R-0011, deferred)

**Open questions to defer to SPEC-0029:**
- Exact Next.js version and `app` vs `pages` router
- Cookie management approach (next-auth vs custom middleware vs iron-session)
- CI/CD: deploy target (Vercel, Cloudflare Pages, self-hosted alongside backend)
- File upload UX: drag-and-drop vs plain input

**Decision log:** Record the Next.js / Tailwind / TypeScript / httpOnly cookie /
no-new-endpoints decisions with today's date (2026-06-21) and rationale.

Mark status **Accepted**.

---

## Step 4 — Commit and push

```bash
git checkout main
git checkout -b R-0029-web-frontend
# write the file
git add requirements/0029-web-frontend.md
git commit -m "R-0029: step-1 requirement — web frontend client (Accepted)"
git push -u origin R-0029-web-frontend
```
