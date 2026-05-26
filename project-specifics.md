# Project Specifics

This is the **single per-project file**. Every other document in this
methodology is generic and identical across all projects — only this file
changes. Fill it in when the project starts; keep it current as these facts
change.

`CLAUDE.md` imports this file, so its contents are always in context.

## Identity

- **Project name:** fitAI
- **One-line description:** Personalized fitness optimization using ML-driven adaptive programming based on individual physiological response.
- **Owner / final decision authority:** Gustavo Delgadillo <gustavo.delgadillo@gmail.com>
- **Repository URL:** https://github.com/westerngazoo/sargentAI
- **Source of requirements:** [`docs/fitness_ai_project.md`](docs/fitness_ai_project.md) — the canonical brief from which `R-NNNN` files are derived.

## Language & toolchain

This project is a **two-stack** system: a Rust backend (intelligence lives
server-side) and a Flutter mobile client (thin — display, logging, photo
capture only). The commands below cover both. Each `R-NNNN` declares which
stack(s) it touches; `qa` and `architect` agents apply the matching gates.

The exact command lines are **defaults**, to be confirmed during M0 when the
workspaces are scaffolded — that confirmation is itself the first wizzielyn
decision recorded against `R-0001`.

### Backend (Rust)

- **Primary language / version:** Rust (stable, edition 2021+; pin in `rust-toolchain.toml`)
- **HTTP framework:** Axum (or Actix-web — chosen in `R-0001`)
- **Async runtime:** Tokio
- **Database client:** `sqlx` against PostgreSQL
- **ML crates (Phase 1):** `linfa`, `ndarray`
- **ML crates (Phase 2, optional):** `burn` or `tch-rs`
- **Build command:** `cargo build --workspace --all-targets`
- **Test command:** `cargo test --workspace --all-features`
- **Lint command:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Format-check command:** `cargo fmt --all -- --check`

### Mobile (Flutter)

- **Primary language / version:** Dart (stable Flutter SDK; pin in `pubspec.yaml`)
- **Framework:** Flutter (iOS + Android, single codebase)
- **Build command:** `flutter build apk --debug` (CI smoke) / `flutter build ipa` and `flutter build appbundle` (release)
- **Test command:** `flutter test`
- **Lint command:** `flutter analyze`
- **Format-check command:** `dart format --set-exit-if-changed .`

### Cross-cutting

- **Repository layout:** **monorepo** — `/backend` (Rust workspace) + `/mobile` (Flutter app), plus this top-level methodology set. Decision recorded against R-0001.
- **Container:** Docker (single Rust API image for MVP)
- **CI/CD:** GitHub Actions → Docker registry → cloud (AWS or Azure — chosen in M8)
- **Object storage:** S3-compatible
- **Auth:** JWT-based (OAuth2 social login optional — see Open Questions in source doc)

## Domain notes

**The intelligence is server-side.** The mobile app is intentionally thin:
log capture, photo capture, dashboard display. No on-device inference. This
is a deliberate choice for the target market — Mexico and LATAM, where Android
hardware quality varies widely — and it lets the ML model improve without
shipping app updates.

**Bodybuilder archetypes as priors.** New users are matched to a curated
"archetype library" — Mentzer (low-volume HIT), Arnold (high-volume split),
Columbu (powerbuilding), Yates (heavy-duty) and similar — to bootstrap a
starting program before the user has logged enough data for personalization.
The archetype is the *prior*; the per-user logs drive the posterior.

**Two model phases.**
- **Phase 1 (MVP):** supervised regression / tree models on structured logs
  using `linfa`. Predicts strength gain and body-composition change; recommends
  adjustments to volume, frequency, intensity, rest, and macros.
- **Phase 2:** sequential / time-series models (`burn` or `tch-rs`) if the
  signal warrants the complexity.

**Photo pipeline feeds the structured model, not the other way around.**
Fixed-angle photos → pose estimation (MediaPipe candidate) → derived features
(shoulder-width proxy, muscle belly visibility, symmetry score) → inputs to
the main model. Raw images never reach the regression model.

**Health data is sensitive.** Photos and biometrics fall under health-data
privacy regimes in most jurisdictions; legal review and a privacy policy are
required before launch (see M8).

## Milestone themes

Mirrored into `ROADMAP.md` once accepted. Proposed sequence:

- **M0** — Foundation (methodology in place, this file complete, toolchain confirmed)
- **M1** — Backend skeleton, auth, user profile
- **M2** — Logging core: workouts, nutrition, photo sessions
- **M3** — Flutter MVP: onboarding, loggers, dashboard
- **M4** — Archetype library and initial-program assignment
- **M5** — ML inference (Phase 1, `linfa`): response inference + adjustment engine
- **M6** — Photo pipeline: pose estimation, derived features, compliance tracking
- **M7** — Subscription, billing, freemium gating
- **M8** — Launch readiness: privacy/legal, store accounts, production deploy
