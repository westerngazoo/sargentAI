# R-0029 — Web Frontend

- **Status:** Accepted
- **Milestone:** M3 (fast-track) / cross-cutting
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-21
- **Depends on:** R-0002 (auth), R-0003 (profile), R-0006 (photo session),
            R-0013 (matching), R-0014 (program+diet)
- **Realized by:** SPEC-0029 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A Next.js web application that gives users browser access to the core fitAI loop: register/login → profile setup → photo upload → archetype matching → program proposals → active program view → workout history.

## 2. Rationale

The owner's stated goal is "ease of use, accessible from a browser." Target users include the same end users on desktop/laptop, plus potentially coaches managing clients. Next.js with React and TypeScript will be used for SSR and file-based routing. Tailwind CSS will be used for styling. It will consume the existing JSON REST API with no new backend endpoints, using the same JWT stored in an httpOnly cookie to avoid XSS.

## 3. Acceptance criteria

- **AC1.** Register + login via the existing API; JWT stored in httpOnly cookie.
- **AC2.** Profile setup form (`PUT /profile/me`).
- **AC3.** Photo upload + match flow (file input → `POST /photo-sessions` → `POST /photo-sessions/:id/photos` → `POST /photo-sessions/:id/match`).
- **AC4.** Proposals screen (top-3 cards, expand, choose).
- **AC5.** Active program detail page.
- **AC6.** Workout history list.
- **AC7.** Responsive layout (mobile-first, works on 375px–1440px).
- **AC8.** Auth-gated routes (redirect to login if no valid session).
- **AC9.** No new backend endpoints — only existing API consumed.
- **AC10.** TypeScript strict mode; `next lint` clean; `tsc --noEmit` clean.
- **AC11.** At minimum smoke tests: render register page, render login page, render home page when mocked as authenticated.
- **AC12.** `next build` succeeds in CI (add to GitHub Actions workflow).

## 4. Constraints & non-goals

- No new backend endpoints
- No server-side rendering of user-specific data without auth (cookie-gated)
- Famous-athlete internal names never rendered in the UI (same rule as mobile)
- No billing/gating UI (M7)
- No nutrition log UI (R-0010, deferred)
- No dashboard/trends (R-0011, deferred)

## 5. Open questions

Deferred to SPEC-0029:
- Exact Next.js version and `app` vs `pages` router
- Cookie management approach (next-auth vs custom middleware vs iron-session)
- CI/CD: deploy target (Vercel, Cloudflare Pages, self-hosted alongside backend)
- File upload UX: drag-and-drop vs plain input

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-21 | **Framework: Next.js (React, TypeScript), Tailwind CSS.** | Standard web stack providing SSR and rapid styling. |
| 2026-06-21 | **Auth: Same JWT stored in httpOnly cookie.** | Secure cookie-based auth prevents XSS exposure while using existing tokens. |
| 2026-06-21 | **No new backend endpoints.** | Consume only existing API to keep scope purely frontend. |

## Changelog

- _2026-06-21 — created and **Accepted**._
