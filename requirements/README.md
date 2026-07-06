# Requirements

A **requirement** states *what* the project must do — a capability or property,
from the problem perspective, independent of implementation. Requirements are
the WHAT; [`specs/`](../specs/) are the HOW.

Every requirement is decided **together** (owner + Claude) before a spec is
written, and every requirement is owned by a `qa` agent run that verifies it.

## Process

1. **Discuss.** Owner and Claude agree the capability and its acceptance
   criteria. See [`CLAUDE.md`](../CLAUDE.md) §4.
2. **Record.** Create a file from [`TEMPLATE.md`](TEMPLATE.md), numbered
   `R-NNNN` (next free 4-digit id): `NNNN-short-name.md`.
3. **Accept.** When acceptance criteria are unambiguous, status → `Accepted`.
   Only then may a spec realize it.
4. **Realize.** One or more `SPEC-NNNN` in `specs/` implement the requirement.
5. **Verify.** The `qa` agent, scoped to this `R-NNNN`, confirms every
   acceptance criterion. Status → `Met`.

## Status values

`Draft` → `Accepted` → `Met` · (or `Superseded`) · (or `Regressed` — was met/
accepted then broken by a later change; points to the requirement that rebuilds
it, e.g. R-0027 → R-0035)

## Relationship to specs

A requirement links forward to the spec(s) that realize it; a spec links back to
the requirement(s) it satisfies. The mapping is maintained in
[`ROADMAP.md`](../ROADMAP.md).

## Index

| Req | Title | Milestone | Status |
|-----|-------|-----------|--------|
| [R-0001](0001-monorepo-scaffold.md) | Monorepo scaffold and CI gates | M1 | Met |
| [R-0002](0002-user-authentication.md) | User authentication (JWT + argon2; Postgres introduced) | M1 | Met |
| [R-0003](0003-user-profile.md) | User profile CRUD (DOB/age, height, weight, sex, goals, body-fat) | M1 | Met |
| [R-0004](0004-workout-log.md) | Workout log (sessions → exercises → sets; reps/weight/RPE; full CRUD) | M2 | Accepted |
| … | _R-0005 … R-0026 exist as files but are not yet listed here — full index reconciliation tracked in issue #56._ | | |
| [R-0027](0027-earbud-guided-training.md) | Earbud-guided training (voice-OUT, media-button advance) | M3 | **Regressed** → R-0035 |
| [R-0030](0030-body-type-picker.md) | Visual body-type picker (synthetic match, no photo) | M3 | Accepted (as-built) |
| [R-0032](0032-voice-assistant.md) | Voice logging assistant (STT → LLM intent → auto-log) | M9 | Accepted (as-built) |
| [R-0035](0035-earbud-handsfree-training.md) | Earbud-guided hands-free training (rebuild of R-0027 transport) | M3 | Accepted |
| [R-0036](0036-voice-reminders.md) | Smart missing-log reminders (split from R-0032) | M9 | Accepted |
