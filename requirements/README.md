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
| [R-0004](0004-workout-log.md) | Workout log (sessions → exercises → sets; reps/weight/RPE; full CRUD) | M2 | Met |
| [R-0005](0005-nutrition-log.md) | Nutrition log (protein/carbs/fat/calories; manual entry) | M2 | Met |
| [R-0006](0006-photo-session.md) | Photo session (multipart upload → `ObjectStore` seam; owner-only access) | M2 | Met |
| [R-0007](0007-flutter-app-shell.md) | Flutter app architecture & auth shell | M3 | Met |
| [R-0008](0008-onboarding.md) | Onboarding flow (body stats, goals, optional details) | M3 | Met |
| [R-0009](0009-live-workout-logger.md) | Live workout logger (program-aware session driver) | M3 | Met |
| [R-0012](0012-archetype-library.md) | Archetype library (curated frame/program/diet priors, provenance) | M4 | Met |
| [R-0013](0013-archetype-matching.md) | Photo → archetype matching (pose-estimation frame features) | M4 | Met |
| [R-0014](0014-program-diet-from-archetype.md) | Program + diet generation from matched archetype | M4 | Met |
| [R-0015](0015-log-aggregation.md) | Time-series training-log aggregation | M5 | Draft (implementation shipped — PR #70) |
| [R-0017](0017-adjustment-engine.md) | Program adjustment engine + Coach card | M5 | Draft (implementation shipped — PR #73) |
| [R-0027](0027-earbud-guided-training.md) | Earbud-guided training (voice-OUT, media-button advance) | M3 | **Regressed** → R-0035 |
| [R-0029](0029-web-frontend.md) | Web frontend client | M3 / cross-cutting | Accepted (in progress — Flutter web build deployed via PR #84) |
| [R-0030](0030-body-type-picker.md) | Visual body-type picker (synthetic match, no photo) | M3 | Accepted (as-built; shipped) |
| [R-0031](0031-nutrition-substitution.md) | Nutrition LLM substitution | M5 | Accepted |
| [R-0032](0032-voice-assistant.md) | Voice logging assistant (STT → LLM intent → auto-log) | M9 | Accepted (as-built; shipped — PRs #39/#49/#50) |
| [R-0033](0033-google-sign-in.md) | Google Sign-In (auth extension) | M3 | Accepted (implementation shipped — PR #49) |
| [R-0035](0035-earbud-handsfree-training.md) | Earbud-guided hands-free training (rebuild of R-0027 transport) | M3 | Accepted (implementation shipped — PR #71) |
| [R-0036](0036-voice-reminders.md) | Smart missing-log reminders (split from R-0032) | M9 | Accepted |
| [R-0037](0037-conversational-voice-intent.md) | Conversational multi-turn voice intent | M9 | Draft (parked — issue #89) |
| [R-0038](0038-periodization-engines.md) | Periodization engines: linear, undulating (DUP), block | M4+ / methodology | Draft (implementation shipped — PR #75) |
| [R-0039](0039-program-authoring.md) | Program authoring model (trainer/self-authored) | M-Platform | Draft (implementation shipped — PR #76) |
| [R-0041](0041-authored-program-persistence.md) | Authored-program persistence + serving (API) | M-Platform | Draft (implementation shipped — PR #85) |
| [R-0042](0042-goal-targets-and-pace.md) | Goal targets & pace tracking | M5 | Draft |
| [R-0043](0043-generated-plan-endpoints.md) | Generated-plan endpoints | M-Platform / M5 | Draft |

Ids without a file: R-0010, R-0011, R-0016, R-0018–R-0026 are roadmap rows not
yet discussed; R-0028 (orphan-object sweep) is deferred in `ROADMAP.md`;
R-0040 (trainer/client roles) exists only as issue #77; R-0034 is unassigned —
the measurements API shipped inside PR #52 without a register entry (owner
decision needed on backfilling a requirement).
