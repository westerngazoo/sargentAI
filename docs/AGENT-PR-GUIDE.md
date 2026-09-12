# Opening a PR against this repo (agents: read this first)

For Cursor agents, Jules, Copilot, and any other automated contributor working
in parallel on this repository.

This repo is governed by [`CLAUDE.md`](../CLAUDE.md), an engineering
constitution. It is stricter than most repos in one specific way, and ignoring
it is why **16 pull requests have been closed unmerged** without anyone
reviewing the code in them.

---

## The one rule that gets PRs closed

> **§1.1 — Nothing is implemented without an accepted requirement and an
> accepted spec.**

A good implementation of a requirement that is not accepted **cannot be
merged**. This is not a style preference; it is the point of the methodology.

### Check before you write any code

```bash
# 1. Does the requirement exist, and what is its Status?
grep -m1 '^- \*\*Status:\*\*' requirements/00NN-*.md

# 2. What does the roadmap say about it?
grep -n 'R-00NN' ROADMAP.md

# 3. Does an accepted spec exist?
ls specs/00NN-*.md
```

Interpret the Status line like this:

| Status | Meaning | May you implement? |
|---|---|---|
| `Draft` | recorded, not agreed | **No** |
| `Backlog` | agreed, not scheduled | **No** |
| `Parked` | deliberately shelved | **No** — and there is a reason written down |
| `Accepted` | requirement agreed, spec may not exist | **Only if `specs/00NN-*.md` exists** |
| `Met` | already shipped | No — unless you are fixing a defect in it |

**R-0037 is Parked.** Sixteen agents have implemented it anyway. If you are
about to work on conversational or multi-turn voice intent: stop, read issue
#89, and do not open a PR.

### The exception: defects

You may fix a defect in a requirement that is already `Met` without a new
requirement, because a defect fix restores behaviour that was already accepted.
Say so explicitly in the PR, and cite the acceptance criterion the current
behaviour violates.

---

## Before you push

Run every gate locally. CI runs all of them and a red PR will not be reviewed.

```bash
# Backend
cd backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Mobile
cd mobile
dart format --output=none --set-exit-if-changed .
flutter analyze
flutter test
```

### Two local traps

**ONNX / `ort-sys` linking.** `cargo test` may fail to link ONNX Runtime on
your machine (issue #55). Clippy with `--all-targets` still *compiles* the
tests, so it catches type errors. If you cannot run the Rust suite, **say so in
the PR** rather than implying it passed.

**The Flutter pin.** `mobile/.flutter-version` is authoritative. If your local
Flutter differs, `pubspec.lock` may resolve differently and CI will reject it.
Do not "fix" a lockfile failure by regenerating the lock until you have
confirmed your SDK matches the pin — CI now asserts this before it reaches the
lockfile step, so read *that* error first.

**Never hand-edit `pubspec.lock`.** Editing version strings without their real
`sha256` hashes has happened here before and is worse than the original
problem.

---

## Branch and PR shape

```
R-00NN-short-name      # work realizing requirement R-00NN
fix/short-name         # a defect fix
ci/short-name          # build, CI or deploy
docs/short-name        # documentation and register
```

**One PR per requirement or per coherent defect.** Small, reviewable diffs.

**Do not stack on another PR's branch** unless you say so prominently in the
description. A stacked PR **auto-closes** when its base branch is deleted —
PR #72 was lost that way, and #104 is currently exposed to it.

### What the description must contain

1. The requirement id, or the defect and the criterion it violates.
2. **What you verified, and how.** Paste the gate output.
3. **What you could NOT verify**, explicitly. An unrun test suite described as
   passing is worse than one described as unrun.
4. Any decision you made that the owner should be able to veto, called out as
   such rather than buried.

---

## Things that will get a PR rejected on sight

- Implementing a `Draft`, `Backlog` or `Parked` requirement.
- Changing an accepted requirement's **acceptance criteria** to match what you
  built. Amendments are owner decisions (§1.2). Correcting a stale `Status`
  line is fine and belongs in that file's changelog.
- Rewriting historical records — merged decision logs, changelogs, and accepted
  specs record what was true when written. They are the audit trail.
- Renaming identifiers in bulk (the Dart package `fitai`, the crates
  `fitai-core`/`fitai-api`, worker names). No user sees them, it conflicts with
  every open branch, and it has been decided against.
- Touching either of these, which silently destroy user state:
  - `token_store.dart`'s `'fitai.jwt'` — no migration shim; renaming logs out
    every install.
  - the Android notification channel **id** in `earbud_coach.dart` — immutable
    per install; renaming orphans the existing channel.
- Fabricating a value where the codebase refuses. The honesty precedent is
  load-bearing: R-0041 returns `null`, R-0042 returns `InsufficientData`,
  R-0044 returns a typed `Refusal`. **A confident wrong answer is worse than no
  answer**, especially anywhere near technique or calories.

---

## Working in parallel without collisions

Several agents work this repo at once. To avoid wasted work:

- **Claim the requirement id in the branch name** and push the branch early,
  even empty. `git ls-remote --heads origin` shows what is taken.
- **Check open PRs first:** `gh pr list --state open`. If one already targets
  your requirement, comment on it instead of opening a rival.
- Prefer **narrow, orthogonal** changes. Two agents touching
  `mobile/lib/src/workout/` will conflict; one on backend and one on mobile
  will not.
- **Do not modify `.github/workflows/`, `project-specifics.md`, `ROADMAP.md` or
  `CLAUDE.md`** unless that is the explicit job. These are shared and conflict
  badly.

---

## Where the truth lives

| File | Answers |
|---|---|
| `ROADMAP.md` | what is next, and current state |
| `requirements/00NN-*.md` | what must be true (acceptance criteria) |
| `specs/00NN-*.md` | how it is built |
| `CLAUDE.md` | to what standard |
| `project-specifics.md` | project identity and toolchain commands |

When they disagree, the requirement's acceptance criteria win over prose, and
the owner wins over everything.
