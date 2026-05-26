# Roadmap

The single source of truth for what is being built and in what order — for the
project named in `project-specifics.md`. Milestones group requirements; each
requirement is realized by one or more specs. Nothing moves without passing the
requirement loop in [`CLAUDE.md`](CLAUDE.md) §4.

## Status legend

`Backlog` → `Discussing` → `Spec'd` → `In progress` → `In review` → `Done`

## Milestones

### M0 — Foundation  ·  *in progress*

Adopt the methodology and prepare the repository.

| Item | Status |
|------|--------|
| Methodology files in place (`CLAUDE.md`, `requirements/`, `specs/`, agents) | Backlog |
| `project-specifics.md` filled in | Backlog |
| Toolchain chosen and recorded | Backlog |
| First requirement discussed | Backlog |

### M1 — <first milestone>

> Replace this with the project's first real milestone. Give it a theme, then
> list the requirements that belong to it.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0001 | <capability> | SPEC-0001 | Backlog |

### M2 — <next milestone>

> Add milestones as the project's scope becomes clear. Keep them small enough
> that each is a coherent, shippable increment.

## Sequencing rules

- A requirement enters `Discussing` only when every requirement it depends on is
  `Done`.
- Requirement and spec ids are 4-digit and shared in spirit: `R-0001` is
  realized by `SPEC-0001` unless a requirement needs several specs.
- This file is updated by the orchestrator whenever a requirement changes state.

## Current focus

Finish M0, then discuss the first requirement (`R-0001`) together before any
spec is written.
