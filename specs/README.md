# Specs

A **spec** states *how* a feature is built — the technical design that realizes
one or more requirements. Requirements
([`requirements/`](../requirements/)) are the WHAT; specs are the HOW.

The project is built spec-first: before any code is written, the feature is
described here as a numbered spec — design, code outline, non-goals, acceptance
mapping — and reviewed by the `architect` agent.

## Process

1. **Draft.** Once the governing requirement is `Accepted`, create a spec from
   [`TEMPLATE.md`](TEMPLATE.md), numbered `SPEC-NNNN`: `NNNN-short-name.md`.
2. **Design review.** The `architect` agent reviews the design and code outline
   against the requirement (`CLAUDE.md` §4, step 2).
3. **Accept.** When the design is sound and unambiguous, status → `Accepted`.
   Only then does implementation begin.
4. **Implement.** Code satisfies exactly the accepted spec and cites its id.
5. **Verify.** Acceptance criteria are checked; status → `Implemented`.

A spec may later become `Superseded` or `Revised` (amended in place, logged).

## Status values

`Draft` → `Accepted` → `Implemented` · (or `Superseded` / `Revised`)

## Relationship to requirements

Every spec links back to the requirement(s) it realizes via its **Realizes**
field. The build order across requirements and specs is in
[`ROADMAP.md`](../ROADMAP.md).

## Index

| Spec | Title | Realizes | Status |
|------|-------|----------|--------|
| [SPEC-0001](0001-monorepo-scaffold.md) | Monorepo scaffold and CI gates | R-0001 | Implemented |
| [SPEC-0002](0002-user-authentication.md) | User authentication (JWT HS256, argon2id, Postgres, `crates/core/` introduced) | R-0002 | Implemented |
| [SPEC-0003](0003-user-profile.md) | User profile CRUD (1:1 `user_profiles`, GET/PUT `/profile/me`, core profile domain) | R-0003 | Implemented |
| [SPEC-0004](0004-workout-log.md) | Workout log (3-table sessions→exercises→sets, full CRUD `/workouts`, first transaction, core workout domain) | R-0004 | Implemented |
| [SPEC-0005](0005-nutrition-log.md) | Nutrition log (model + REST endpoints, manual entry) | R-0005 | Implemented |
| [SPEC-0006](0006-photo-session.md) | Photo session (multipart upload, `ObjectStore` seam, compensation path) | R-0006 | Implemented |
| [SPEC-0007](0007-flutter-app-shell.md) | Flutter app architecture & auth shell | R-0007 | Implemented |
| [SPEC-0008](0008-onboarding.md) | Onboarding flow (wizard over `PUT /profile/me`) | R-0008 | Implemented |
| [SPEC-0009](0009-live-workout-logger.md) | Live workout logger (`SessionDriver`, preset picker) | R-0009 | Implemented |
| [SPEC-0012](0012-archetype-library.md) | Archetype library (embedded typed-Rust, authenticated read API) | R-0012 | Implemented |
| [SPEC-0013](0013-archetype-matching.md) | Photo → archetype matching (in-process ONNX pose → ranked archetypes) | R-0013 | Implemented |
| [SPEC-0014](0014-program-diet-from-archetype.md) | Program + diet generation from matched archetype | R-0014 | Implemented |
| [SPEC-0015](0015-log-aggregation.md) | Training-log aggregation core | R-0015 | Draft (implementation shipped — PR #70) |
| [SPEC-0017](0017-adjustment-engine.md) | Adjustment engine + summary/adjustments endpoints + Coach card | R-0017 | Draft (implementation shipped — PR #73) |
| [SPEC-0027](0027-earbud-guided-training.md) | Earbud-guided training (as-built audit; documents the regression) | R-0027 | Accepted (as-built) |
| [SPEC-0030](0030-body-type-picker.md) | Visual body-type picker | R-0030 | Accepted (implementation shipped) |
| [SPEC-0032](0032-voice-assistant.md) | Voice logging assistant (voice hub) | R-0032 | Accepted (implementation shipped — PRs #39/#49/#50) |
| [SPEC-0035](0035-earbud-handsfree-training.md) | Earbud-guided hands-free training (transport rebuild) | R-0035 | Draft (implementation shipped — PR #71) |
| [SPEC-0038](0038-periodization-engines.md) | Periodization engines: linear, undulating (DUP), block | R-0038 | Draft (implementation shipped — PR #75) |
| [SPEC-0039](0039-program-authoring.md) | Program authoring model | R-0039 | Draft (implementation shipped — PR #76) |
| [SPEC-0041](0041-authored-program-persistence.md) | Authored-program persistence + serving (API) | R-0041 | Draft (implementation shipped — PR #85) |
