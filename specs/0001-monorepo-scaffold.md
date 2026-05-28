# SPEC-0001 — Monorepo scaffold and CI gates

- **Status:** Accepted
- **Realizes:** R-0001
- **Author:** Claude (main session), with owner
- **Created:** 2026-05-27
- **Revised:** 2026-05-28 (after architect review)
- **Depends on:** none
- **Module(s):** `/backend/crates/api`, `/mobile/`, `/.github/workflows/`, `/backend/Dockerfile`

## 1. Motivation

Realizes [R-0001](../requirements/0001-monorepo-scaffold.md): a two-stack
monorepo with a minimal Axum service, a minimal Flutter app, a Dockerfile that
runs the backend, and a GitHub Actions workflow that enforces the merge gates
on every push. R-0001's eight acceptance criteria map 1:1 to the verification
checklist in §6.

## 2. Design

### 2.1 Repository layout

```
fitAI/
├── .github/workflows/ci.yml
├── .gitignore
├── CLAUDE.md
├── ROADMAP.md
├── project-specifics.md
├── README.md                       # top-level orientation (new in this spec)
├── docs/fitness_ai_project.md      # the source brief
├── requirements/…
├── specs/…
├── .claude/agents/…
├── backend/
│   ├── Cargo.toml                  # [workspace] members = ["crates/api"]
│   ├── Cargo.lock                  # committed; CI runs with --locked
│   ├── rust-toolchain.toml         # pins channel + components
│   ├── Dockerfile
│   ├── .dockerignore
│   └── crates/
│       └── api/
│           ├── Cargo.toml          # name = "fitai-api"
│           ├── src/
│           │   ├── lib.rs          # pub fn app() -> Router (testable)
│           │   ├── main.rs         # bin: bind, serve, graceful shutdown
│           │   └── health.rs       # GET /health -> 200
│           └── tests/
│               └── health.rs       # two tests: router-only + real-server boot
└── mobile/
    ├── pubspec.yaml
    ├── analysis_options.yaml
    ├── .flutter-version            # plain-text Flutter SDK pin (subosito reads this)
    ├── lib/
    │   ├── main.dart
    │   ├── app.dart
    │   └── screens/home_screen.dart
    └── test/home_screen_test.dart
```

### 2.2 Backend crate strategy

**Single crate `fitai-api` now.** No empty `fitai-core` yet — adding a crate
before a domain type exists is YAGNI per `CLAUDE.md` §2. The split happens at
the next requirement that introduces a domain type, which is **R-0003**
(user profile); SPEC-0003 must introduce `crates/core/` and pull the profile
types out of `api`. This trigger is recorded in §7.

### 2.3 Router and testability

`lib.rs` exposes `pub fn app() -> Router`. `main.rs` is the binary that calls
`app()`, binds a `TcpListener`, and runs `axum::serve` with graceful shutdown.
Two integration tests live under `tests/health.rs`:

1. **`health_returns_ok_via_router`** — calls `app()` directly and exercises
   the route via `tower::ServiceExt::oneshot`. Fast, no port binding.
2. **`health_returns_ok_via_real_server`** — binds `127.0.0.1:0`, spawns
   `axum::serve` in a task, issues a real `reqwest` GET, asserts 200, and
   aborts the task. This is the test that satisfies R-0001 AC2's "boots the
   HTTP service" wording literally; the router-only test guards against
   regressions cheaply on every run.

### 2.4 Server boot

- `PORT` env (default `8080`); bind `0.0.0.0:$PORT`.
- `tracing` initialized from `RUST_LOG`.
- Graceful shutdown on `ctrl_c` *and* SIGTERM (the latter is what Docker / k8s
  send on `docker stop`).
- **No `.unwrap()` or `.expect()`.** Signal-handler install errors propagate
  via `?` from `main` (it already returns `Result`); the `ctrl_c` future's
  own `io::Result` is logged and shutdown proceeds. See §7 for the rationale.

### 2.5 Dockerfile strategy

Multi-stage: `rust:<pinned>-slim-bookworm` builder → `debian:bookworm-slim`
runtime. Non-root user (`uid 10001`). `ca-certificates` installed in runtime
(so future HTTPS clients work without surprise). Built binary copied to
`/usr/local/bin/fitai-api`. `EXPOSE 8080`, `CMD ["fitai-api"]`. Distroless
explicitly deferred to R-0026.

**No `pkg-config libssl-dev` in the builder stage.** The current dependency
set has no `openssl-sys` consumer; we add those packages when a spec
introduces a dep that needs them (likely SPEC-0002 / SPEC-0004 with `sqlx`
or `reqwest`-with-`native-tls`). The current dev-dep `reqwest` uses
`rustls-tls` precisely so the builder image stays apt-free.

**No `HEALTHCHECK` directive.** `curl` is not in `debian-slim` and adding it
just for `HEALTHCHECK` is excess for R-0001. AC7 is satisfied by external
probing. `HEALTHCHECK` is explicitly deferred to **R-0026** (production
deployment), which will pick the runtime probing approach (k8s livenessProbe,
container HEALTHCHECK, or both) holistically.

**No dependency-only pre-fetch layer in the Dockerfile.** Conventional Rust
two-step layer caching (copy `Cargo.toml` + `Cargo.lock`, `cargo fetch`,
then copy `src/`) is *not* added in this spec. R-0001's dep tree is small
(~30 transitive crates) and the cold build runs in well under 5 minutes.
**Trigger to revisit:** when total CI wall time on the `docker` job exceeds
**5 minutes** on the `main` branch, file an extension R against R-0026
to introduce dep-pre-fetch + buildx layer cache.

### 2.6 Mobile app strategy

The smallest meaningful Flutter app: `MaterialApp` → `Scaffold` → centred
`Text('fitAI')`. One widget test pumps `HomeScreen` and asserts the text is
visible. `flutter_lints` provides the lint baseline. The Flutter SDK is pinned
in a plain-text `mobile/.flutter-version` file; CI reads it natively via
`subosito/flutter-action@v2`'s `flutter-version-file` input. Devs run plain
`flutter` and are responsible for matching the pin (no third-party version
manager is required — keeps the dev setup minimal).

### 2.7 CI graph

Three parallel jobs on `ubuntu-latest`; all three must pass for merge (branch
protection is configured outside this repo, but the workflow exposes the
required statuses).

| Job | Purpose | Cache |
|-----|---------|-------|
| `rust` | fmt → clippy → test → build, all `--locked` (incl. clippy) | `Swatinem/rust-cache@v2` keyed on `backend/Cargo.lock` |
| `mobile` | version-file → format-check → analyze → test | flutter-action default cache |
| `docker` | `docker build` against `/backend/Dockerfile` | none (see §2.5 trigger) |

Triggers: `push` on any branch, `pull_request` targeting `main`.
`concurrency` cancels in-progress runs **only for non-`main` refs** so that
fast-follow merges to `main` cannot leave the default branch without a
recorded green build.

### 2.8 Pinned versions

The exact pins are committed at implementation time and recorded in §7. The
implementer runs `rustup show` / `flutter --version` against the host, pins to
the installed stables, and runs `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` against the scaffolded code, adjusting
any snippet in §3 that fails the gate in lockstep, before the PR.

| Tool | Pin (resolved 2026-05-28) | Pinned in |
|------|---------------------------|-----------|
| Rust | `1.95.0` | `backend/rust-toolchain.toml` |
| Flutter | `3.44.0` | `mobile/.flutter-version` |
| Axum | `0.7` | `backend/Cargo.toml` (`[workspace.dependencies]`) |
| Tokio | `1` | `backend/Cargo.toml` |
| reqwest (dev) | `0.12` | `backend/Cargo.toml` |

## 3. Code outline

The files below are the **agreed implementation shape** per `CLAUDE.md` §4.4.
The implementation step (step 5) commits them with the pin substitutions
noted in §2.8 and the clippy-clean verification noted in §7.

### 3.1 `backend/Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["crates/api"]

[workspace.package]
edition = "2021"
license = "UNLICENSED"
publish = false

[workspace.dependencies]
axum = "0.7"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "net"] }
tower = "0.5"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
http-body-util = "0.1"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }

[workspace.lints.rust]
unsafe_code = "forbid"
unreachable_pub = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
```

### 3.2 `backend/rust-toolchain.toml`

```toml
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

### 3.3 `backend/crates/api/Cargo.toml`

```toml
[package]
name = "fitai-api"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish.workspace = true   # inherits workspace value (false)

[lib]
path = "src/lib.rs"

[[bin]]
name = "fitai-api"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
axum.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
tower = { workspace = true, features = ["util"] }
http-body-util.workspace = true
reqwest.workspace = true
```

### 3.4 `backend/crates/api/src/lib.rs`

```rust
//! fitai-api library entry. Exposes the router so tests don't bind a port.
//!
//! Inside `#[cfg(test)]` (unit tests in this crate) the strict
//! `clippy::unwrap_used`/`expect_used`/`panic` lints are relaxed — test
//! code is the conventional place for those. Integration tests under
//! `tests/` are separate crates and each opt out at file top.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod health;

use axum::Router;

/// Build the application router.
///
/// `main.rs` wraps this with `axum::serve`. Tests call it directly via
/// `tower::ServiceExt::oneshot` or boot a real server in a task.
///
/// (`Router` is itself `#[must_use]`, so no attribute here — `clippy::double_must_use`.)
pub fn app() -> Router {
    Router::new().merge(health::router())
}
```

### 3.5 `backend/crates/api/src/health.rs`

```rust
//! `GET /health` — the minimum readiness signal.

use axum::{http::StatusCode, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> StatusCode {
    StatusCode::OK
}
```

### 3.6 `backend/crates/api/src/main.rs`

```rust
//! fitai-api binary: bind, serve, shut down gracefully.
//!
//! No `.unwrap()` / `.expect()`. Signal-handler install failures propagate
//! via `?` (the same way port-bind failures do); the `ctrl_c` future's own
//! `io::Result` is logged and shutdown proceeds (we'd rather shut down
//! cleanly than abort the process on a `ctrl_c` handler hiccup).

use std::net::SocketAddr;
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "fitai-api listening");

    let shutdown = build_shutdown()?;

    axum::serve(listener, fitai_api::app())
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(())
}

/// Install signal handlers up-front and return a future that resolves when
/// any of them fires. Returning `Err` here is unrecoverable — without
/// signal handling the process cannot gracefully drain, which would corrupt
/// shutdown semantics for `docker stop` / k8s rolling deploys.
fn build_shutdown() -> Result<impl std::future::Future<Output = ()>, std::io::Error> {
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    Ok(async move {
        #[cfg(unix)]
        {
            tokio::select! {
                r = ctrl_c() => log_ctrl_c_error(r),
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            log_ctrl_c_error(ctrl_c().await);
        }
        tracing::info!("shutdown signal received");
    })
}

fn log_ctrl_c_error(r: std::io::Result<()>) {
    if let Err(e) = r {
        tracing::warn!(error = %e, "ctrl_c handler error; shutting down anyway");
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}
```

### 3.7 `backend/crates/api/tests/health.rs`

```rust
//! Integration tests for `GET /health`.
//!
//! Two tests: one via the in-process router (fast, no port), one via a
//! real `axum::serve` boot on `127.0.0.1:0` (literal AC2: "boots the
//! HTTP service in-process"). Both must pass.

#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok_via_router() {
    let app = fitai_api::app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty(), "health body should be empty");
}

#[tokio::test]
async fn health_returns_ok_via_real_server() {
    // Bind ephemeral port, capture address, hand listener to axum::serve.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, fitai_api::app()).await.unwrap();
    });

    let url = format!("http://{addr}/health");
    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    server.abort();
}
```

### 3.8 `backend/Dockerfile`

```dockerfile
# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p fitai-api --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 app
USER app
COPY --from=builder /src/target/release/fitai-api /usr/local/bin/fitai-api
EXPOSE 8080
ENV PORT=8080
CMD ["fitai-api"]
```

### 3.9 `backend/.dockerignore`

```
target/
**/*.bak
.git/
.dockerignore
Dockerfile
*.md
.DS_Store
```

### 3.10 `mobile/pubspec.yaml`

```yaml
name: fitai
description: "fitAI mobile client — thin: capture, display, no on-device inference."
publish_to: 'none'
version: 0.1.0+1

environment:
  sdk: '>=3.5.0 <4.0.0'
  flutter: '>=3.44.0'

dependencies:
  flutter:
    sdk: flutter

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^5.0.0

flutter:
  uses-material-design: true
```

### 3.11 `mobile/.flutter-version`

Plain text, one line, no trailing newline-of-art. `subosito/flutter-action@v2`
reads this directly via its `flutter-version-file` input.

```
3.44.0
```

### 3.12 `mobile/analysis_options.yaml`

```yaml
include: package:flutter_lints/flutter.yaml
```

### 3.13 `mobile/lib/main.dart`

```dart
import 'package:flutter/material.dart';
import 'app.dart';

void main() {
  runApp(const FitAiApp());
}
```

### 3.14 `mobile/lib/app.dart`

```dart
import 'package:flutter/material.dart';
import 'screens/home_screen.dart';

class FitAiApp extends StatelessWidget {
  const FitAiApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'fitAI',
      home: const HomeScreen(),
    );
  }
}
```

### 3.15 `mobile/lib/screens/home_screen.dart`

```dart
import 'package:flutter/material.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Text('fitAI'),
      ),
    );
  }
}
```

### 3.16 `mobile/test/home_screen_test.dart`

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:fitai/screens/home_screen.dart';

void main() {
  testWidgets('renders fitAI placeholder', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    expect(find.text('fitAI'), findsOneWidget);
  });
}
```

### 3.17 `.github/workflows/ci.yml`

```yaml
name: ci

on:
  push:
    branches: ['**']
  pull_request:
    branches: [main]

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  # Don't cancel main runs; a fast-follow merge must still record a green build.
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}

jobs:
  rust:
    name: rust (fmt, clippy, test, build)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: backend
    steps:
      - uses: actions/checkout@v4
      - name: install toolchain
        run: rustup show  # reads rust-toolchain.toml
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
      - run: cargo test --workspace --all-features --locked
      - run: cargo build --workspace --all-targets --locked

  mobile:
    name: mobile (analyze, test)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: mobile
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version-file: mobile/.flutter-version
          channel: stable
      - run: flutter pub get
      - run: dart format --set-exit-if-changed .
      - run: flutter analyze
      - run: flutter test

  docker:
    name: docker build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - name: build backend image
        run: docker build -t fitai-api:ci backend
```

### 3.18 `README.md` (top-level, minimal)

```markdown
# fitAI

Personalized fitness optimization using ML-driven adaptive programming.

This repo follows the [wizzielyn](https://github.com/.../wizzielyn) SDLC. Start
at [`CLAUDE.md`](CLAUDE.md), then [`ROADMAP.md`](ROADMAP.md).

## Layout

- `backend/` — Rust workspace (Axum HTTP service; ML server-side)
- `mobile/` — Flutter app (thin client; iOS + Android)
- `docs/` — source brief and design notes
- `requirements/`, `specs/` — wizzielyn requirement and spec files
```

## 4. Non-goals

- No domain types (user, workout, nutrition, photo) — those arrive R-0003+.
- No database, no migrations, no `sqlx` dependency in `Cargo.toml`.
- No auth, no middleware beyond Axum's defaults.
- No release-style Flutter build (APK/IPA/appbundle) in CI — R-0025/R-0026.
- No image registry push, no cloud deploy — R-0026.
- No Dockerfile `HEALTHCHECK` directive — deferred to R-0026 with the
  runtime-probe decision (k8s livenessProbe vs container HEALTHCHECK).
- No Dockerfile dependency-pre-fetch layer or buildx GHA cache — accepted
  cold-build cost; revisit trigger in §2.5.
- No `cargo-deny`, `cargo-audit`, `cargo-machete`, or other supply-chain
  tools — they're appropriate but add merge-gate complexity that should
  land with a spec dedicated to security tooling.
- No `flutter_lints` overrides; the default ruleset is the baseline.

## 5. Open questions

None remaining. R-0001 §6 settled the requirement-level OQs; the architect
review of 2026-05-27 raised the design-level items, all resolved in §7.
Implementer mechanical choices (exact Rust / Flutter version pins, any
snippet adjustments needed to satisfy `clippy::pedantic`) are recorded in
the changelog at implementation time per §2.8 / §7.

## 6. Acceptance criteria

Each maps back to an R-0001 AC; each becomes one or more `qa` agent tests.

- [ ] **SAC1 → AC1.** `cd backend && cargo build --workspace --all-targets` exits 0 against a fresh checkout.
- [ ] **SAC2 → AC2.** `cd backend && cargo test --workspace --all-features --locked` exits 0; both `crates/api/tests/health.rs::health_returns_ok_via_router` and `health_returns_ok_via_real_server` pass. The second test literally boots `axum::serve` on `127.0.0.1:0` and exercises `GET /health` over real HTTP.
- [ ] **SAC3 → AC3.** `cd backend && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` exits 0 with no warnings. Workspace lints include `clippy::pedantic`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`.
- [ ] **SAC4 → AC4.** `cd backend && cargo fmt --all -- --check` exits 0.
- [ ] **SAC5 → AC5.** `cd mobile && flutter analyze` exits 0; `flutter test` exits 0 with `home_screen_test.dart::renders fitAI placeholder` passing.
- [ ] **SAC6 → AC6.** `cd mobile && dart format --set-exit-if-changed .` exits 0.
- [ ] **SAC7 → AC7.** `docker build -t fitai-api:ci backend` succeeds; `docker run --rm -p 8080:8080 fitai-api:ci &` followed by `curl -sf -o /dev/null -w "%{http_code}" localhost:8080/health` prints `200`.
- [ ] **SAC8 → AC8.** `.github/workflows/ci.yml` exists with three jobs (`rust`, `mobile`, `docker`); the R-0001 PR branch shows all three green at the time of `qa` sign-off.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-27 | **Single `fitai-api` crate now.** Trigger to introduce `fitai-core` is R-0003 (first domain type). | YAGNI per `CLAUDE.md` §2; explicit trigger so the inward-dependency rule (§2) is honored when it starts mattering. Owner-approved. |
| 2026-05-27 | **`pub fn app() -> Router` in `lib.rs`.** | Lets integration tests call the router via `oneshot` and lets the second test spawn `axum::serve` over the same builder. Standard Axum idiom. |
| 2026-05-27 | **Graceful shutdown on SIGTERM as well as ctrl_c.** | Docker `stop` sends SIGTERM; without this the container is SIGKILLed after the 10 s grace, slowing redeploys and dropping in-flight requests. |
| 2026-05-27 | **Runtime base image: `debian:bookworm-slim`.** | Easier incident debugging (has a shell, `apt`, `coreutils`). Image still <100 MB. Distroless deferred to R-0026. Owner-approved. |
| 2026-05-27 | **CI is three parallel jobs (`rust`, `mobile`, `docker`).** | Parallelism halves wall time vs sequential; one job per stack matches the merge-gate split in `project-specifics.md`. |
| 2026-05-27 | **`fvm` pins the Flutter SDK; CI reads it via `kuhnroyal/flutter-fvm-config-action@v2`.** | Single source of truth for the Flutter version shared by devs and CI. Default accepted in R-0001 §6. (Updated 2026-05-28 to use the modern `.fvmrc` format.) |
| 2026-05-27 | **Full implementation snippets included in the spec (§3).** | Owner-approved; matches `CLAUDE.md` §4.4 strongly: step 5 implementation is a copy + verify, not invention. |
| 2026-05-27 | **Exact Rust + Flutter version pins resolved at implementation time** by running `rustup show` / `flutter --version` against the host. Recorded in this file's changelog. | Avoids guessing a stable version that may not be current on the implementation day. |
| 2026-05-28 | **Architect finding #1 — `clippy::pedantic` snippet risk:** implementer verifies clippy-clean at scaffold time against the actual code; any snippet that fails the gate is patched in lockstep, in this spec, before the PR opens. The spec is the source of truth — diverging implementation without spec update is a process violation. | Keeps `pedantic` strict from day one; the safety net survives. Owner-approved. |
| 2026-05-28 | **Architect finding #2 — no `.unwrap()` / `.expect()` in `main.rs`:** signal-handler install errors propagate via `?` (`main` returns `Result`); the `ctrl_c` future's own `io::Result` is logged and shutdown proceeds. See §3.6. | Honors `CLAUDE.md` §6 in spirit and in fact. The bin/lib distinction doesn't license abort-on-startup-error here because the project's constitution treats abort as last resort. |
| 2026-05-28 | **Architect finding #4 — AC2 literal compliance:** added a second integration test (`health_returns_ok_via_real_server`) that binds `127.0.0.1:0`, spawns `axum::serve`, and issues a real `reqwest` GET. Router-only test retained as a fast guard. | Removes the AC2 wording ambiguity completely. QA verifies a literal boot. Owner-approved. |
| 2026-05-28 | **Architect finding #5 — workspace lints strengthened:** added `clippy::unwrap_used = "warn"`, `clippy::expect_used = "warn"`, `clippy::panic = "warn"` to `[workspace.lints.clippy]`. Test code opts out via `#![cfg_attr(test, allow(...))]` at lib/main crate root and `#![allow(clippy::unwrap_used)]` at integration-test file top. | Enforces `CLAUDE.md` §6 by toolchain, not vigilance. The lint set arrives in R-0001 so it covers every future R from day one. Owner-approved. |
| 2026-05-28 | **Architect finding #6 — `--locked` on clippy too.** | Otherwise stale `Cargo.lock` silently regenerates and lockfile drift hides. |
| 2026-05-28 | **Architect finding #7 — accept Docker cold-build cost for MVP** with a §2.5 revisit trigger: file an extension R when total `docker` job wall time on `main` exceeds 5 minutes. | Conventional dep-prefetch layer adds ~10 Dockerfile lines and a refactor risk for a problem that doesn't bite at R-0001's dep count. Owner-approved. |
| 2026-05-28 | **Architect finding #8 — drop `pkg-config libssl-dev` from the builder stage.** Dev-dep `reqwest` uses `rustls-tls` precisely so no apt deps are needed for the builder. | No dependency in the binary currently links libssl; carrying apt packages "for the future" violates `CLAUDE.md` §2 ("no premature anything"). |
| 2026-05-28 | **Architect finding #9 — no `HEALTHCHECK` directive in this spec; deferred to R-0026.** | `curl` isn't in `debian-slim`; adding it just to satisfy a `HEALTHCHECK` adds layer weight. R-0026 will choose the right runtime probing approach (k8s livenessProbe vs container HEALTHCHECK) holistically. |
| 2026-05-28 | **Architect finding #10 — `apt-get install` versions float (acceptable for MVP).** | `--no-install-recommends` already in place. Pinning Debian package versions adds maintenance load (renovate-bot territory) disproportionate to R-0001's surface area. |
| 2026-05-28 | **Architect finding #11 — switch from deprecated `.fvm/fvm_config.json` to modern `.fvmrc`.** | fvm 3.x writes `.fvmrc`; the action supports both, but committing the modern format avoids drift the first time a dev runs `fvm use --force` against the project. |
| 2026-05-28 | **Architect finding #15 — `cancel-in-progress` is now conditional on `github.ref != 'refs/heads/main'`.** | Prevents a fast-follow merge to `main` from leaving the default branch without a recorded green build. |
| 2026-05-28 | **Forward-looking, per architect Notes:** SPEC-0026 (production deploy) is the natural home for image-registry push, cosign signing, SBOM generation, runtime probes, and (if needed by then) Dockerfile dep-prefetch + buildx cache. Recording the trigger here so it isn't lost. | Keeps R-0001 minimal while preserving the architectural intent. |
| 2026-05-28 | **Drop `fvm`; pin Flutter via plain `mobile/.flutter-version` read by `subosito/flutter-action@v2`'s `flutter-version-file` input.** Supersedes the earlier `.fvmrc` choice. | One less tool dependency for devs (no `fvm` install required); single action handles version-read + install; CI/dev parity preserved via the committed file. Owner-approved at step 4 (2026-05-28). |
| 2026-05-28 | **Version pins resolved against the implementer host:** Rust **1.95.0**, Flutter **3.44.0**. These are the installed stables on the implementation machine and now appear verbatim in `rust-toolchain.toml`, `mobile/.flutter-version`, the `Dockerfile` `ARG RUST_VERSION`, and the `pubspec.yaml` `flutter:` minimum. | Honors §2.8's "implementer pins to current stable" instruction. Owner-approved at step 4 (2026-05-28). |
| 2026-05-28 | **AC7 (`docker build` + container probe) first runs on CI.** Docker is not installed on the implementation host; local step-5 verification covers AC1–AC6 only. If the `docker` CI job is red on the first PR run, follow-up commits to the same PR fix it. | Pragmatic: standard remote-CI verification path. Owner-approved at step 4 (2026-05-28). |
| 2026-05-28 | **Step-5 lockstep fixes** (architect finding #1 disposition activated). Four mechanical snippet adjustments made in lockstep across §3 and the actual files: (a) `main.rs:39` — `let mut sigterm = …` collapsed onto one line (rustfmt); (b) `lib.rs:19` — removed redundant `#[must_use]` on `pub fn app() -> Router` (`Router` is itself `#[must_use]`; `clippy::double_must_use`); (c) `main.rs:4,6` — wrapped two bare `ctrl_c` mentions in backticks (`clippy::doc_markdown`); (d) `pubspec.yaml:2` — quoted the description string because the embedded `:` after "thin" tripped the YAML parser. All four fixes were necessary to take AC2–AC6 from "compiles" to "fully green". | Each is exactly the kind of small adjustment §2.8 + architect finding #1 anticipated. Spec snippets remain the source of truth — verified clippy-clean. |

## Changelog

- _2026-05-27 — created (Draft); pending `architect` agent review._
- _2026-05-28 — revised after architect review (REQUEST CHANGES verdict resolved): all 2 blocking + 5 major + 4 actionable minor findings addressed; 3 minor findings accepted with rationale in §7. Pending owner re-acceptance._
- _2026-05-28 — owner accepted the revised spec. Status → Accepted. Step 3 (qa test plan) may begin._
- _2026-05-28 — step 4 (code outline review): pins resolved to Rust 1.95.0 / Flutter 3.44.0 against the implementer host; `fvm` dropped in favor of `mobile/.flutter-version` + `subosito/flutter-action@v2`'s `flutter-version-file` input; AC7 deferred to first CI run. Snippets in §3.2 / §3.8 / §3.10 / §3.11 / §3.17 updated in lockstep. Decision log entries added._
- _2026-05-28 — step 5 (implement): all 16 production files written per §3. Lockstep snippet fixes applied (see §7 entry) for `main.rs:39`, `lib.rs:19`, `main.rs:4/6`, `pubspec.yaml:2`. Local gates green: AC1 ✓ AC2 ✓ AC3 ✓ AC4 ✓ AC5 ✓ AC6 ✓ AC8 (script-level) ✓. AC7 deferred to CI run on the PR._
