# fitAI

Personalized fitness optimization using ML-driven adaptive programming.

This repo follows the **wizzielyn** SDLC — read [`CLAUDE.md`](CLAUDE.md) for
the engineering constitution, then [`ROADMAP.md`](ROADMAP.md) for what's being
built and in what order.

## Layout

- `backend/` — Rust workspace (Axum HTTP service; ML server-side)
- `mobile/` — Flutter app (thin client; iOS + Android)
- `docs/` — source brief and design notes
- `requirements/`, `specs/` — wizzielyn requirement and spec files
- `scripts/qa/` — per-requirement QA verification scripts
- `.claude/agents/` — orchestrator, architect, qa subagents

## Pinned toolchains

- Rust **1.95.0** (`backend/rust-toolchain.toml`)
- Flutter **3.44.0** (`mobile/.flutter-version`)
