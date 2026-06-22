# Parallel Agent Tasks

Each section below is a self-contained prompt for a separate Claude Code session.
Hand each one to its own agent. They all work in isolation — no conflicts with
the main session (which is driving the R-0027 earbud loop).

Results come back as files on the `R-0027-earbud-training` branch or their own
branches as indicated. The main session reviews everything before it merges.

---

## TASK 1 — Draft SPEC-0027 (Earbud-guided training)

**Branch:** `R-0027-earbud-training` (already exists, push to it)
**Output:** `specs/0027-earbud-guided-training.md`
**Urgency:** Highest — main session needs this next (Step 2 of the R-0027 loop)

### Prompt for agent

You are writing the technical spec (SPEC-0027) for the fitAI project.

**Project:** Rust backend + Flutter mobile. Repo: `westerngazoo/sargentAI`.
Working directory: `/Users/goose/projects/fitAI`.

**What to do:**
1. Read `requirements/0027-earbud-guided-training.md` — this is the accepted
   requirement. Every AC in it must be addressed in the spec.
2. Read `specs/0013-archetype-matching.md` and `specs/0014-program-diet-from-archetype.md`
   for spec format and depth conventions.
3. Read `mobile/lib/src/workout/` — understand the existing `SessionDriver`,
   `LiveSessionScreen`, and `WorkoutRepository` before designing the voice layer.
4. Read `mobile/pubspec.yaml` — see which packages are already present.

**Decisions already locked (do not re-open):**
- TTS engine: `flutter_tts`
- Media-button + background audio: `audio_service`
- Background audio is a hard requirement (both iOS and Android)
- Single earbud button press = log current set + advance
- Voice mode entry: toggle on `LiveSessionScreen` (not a separate screen)
- Flutter-only — no new backend endpoints, no new migrations

**Spec must cover:**
- §1 Overview (what this builds on top of R-0009's `SessionDriver`)
- §2 Flutter architecture: how `audio_service`, `flutter_tts`, and the existing
  `SessionDriver` wire together; the `VoiceSessionAdapter` (or equivalent) class
  design; state machine for voice mode on/off
- §3 iOS specifics: `AVAudioSession` category, `Info.plist` background mode,
  `MPRemoteCommandCenter` registration
- §4 Android specifics: foreground service declaration, `AndroidManifest.xml`
  changes, `KEYCODE_HEADSETHOOK` handling
- §5 TTS script: exact strings for exercise announcement, set announcement,
  rest cue, workout done (matching AC2/AC3/AC5/AC6)
- §6 Error handling: TTS fail, earbud disconnect, audio focus loss
- §7 Test plan hooks: what the widget tests and unit tests will stub/mock
- §8 Dependency additions to `pubspec.yaml`
- Resolve all five OQ-H questions from the requirement's §5

**Output:** Write `specs/0027-earbud-guided-training.md` on branch
`R-0027-earbud-training`. Commit with message:
`R-0027: SPEC-0027 draft — earbud-guided training`

---

## TASK 2 — Draft Requirement R-0029: Web Frontend Client

**Branch:** create `R-0029-web-frontend` from `main`
**Output:** `requirements/0029-web-frontend.md`

### Prompt for agent

You are writing a new requirement file for the fitAI project.

**Project:** Rust (Axum) backend API + Flutter mobile client.
Working directory: `/Users/goose/projects/fitAI`.

**What to do:**
1. Read `requirements/0014-program-diet-from-archetype.md` for the requirement
   file format to follow exactly.
2. Read `CLAUDE.md` for the engineering constitution.
3. Read `project-specifics.md` for project identity and tech stack.
4. Read `backend/crates/api/src/lib.rs` to understand all existing API routes.
5. Draft `requirements/0029-web-frontend.md`.

**Context:** The backend is a standard JSON REST API (Axum, JWT auth). The
Flutter app is one client — a web frontend would be another client consuming the
same endpoints. The owner wants a web frontend so the product is accessible
from a browser (gym owners, coaches, users on desktop). The web client should
cover the core loop: register/login, photo upload for archetype matching, view
program + diet, view workout history, view current program. Nutrition log UI and
dashboard are already deferred (R-0010/R-0011) and remain deferred.

**Technology recommendation to evaluate and record:**
- Next.js (React, TypeScript) — SSR for SEO, API routes for token proxying
- Tailwind CSS
- Same JWT auth the mobile app uses
- No new backend endpoints needed for MVP (all exist already)
- Mobile repo stays Flutter; web is a new `/web` directory in the monorepo

**Write the requirement file including:**
- Statement (what the web frontend does)
- Rationale (why — owner stated "ease of use, web accessible")
- 8–12 acceptance criteria covering: auth, photo upload, proposals screen,
  program detail, workout history, responsive layout, no new backend endpoints
- Constraints (no new API endpoints in v1 — consume what exists)
- Technology decision section (Next.js / Tailwind / TypeScript — log as decided)
- Open questions section (any HOW-level decisions to defer to SPEC-0029)
- Decision log

Mark status **Accepted** (owner approved this in the step-1 discussion).

Checkout branch `R-0029-web-frontend` from `main`, write the file, commit:
`R-0029: step-1 requirement — web frontend client (Accepted)`

---

## TASK 3 — Draft Requirement R-0030: Visual Body-Type Picker

**Branch:** create `R-0030-body-type-picker` from `main`
**Output:** `requirements/0030-body-type-picker.md`

### Prompt for agent

You are writing a new requirement file for the fitAI project.

**Project:** Rust backend + Flutter mobile.
Working directory: `/Users/goose/projects/fitAI`.

**What to do:**
1. Read `requirements/0014-program-diet-from-archetype.md` for format.
2. Read `requirements/0013-archetype-matching.md` to understand the current
   photo-based matching flow (R-0013) that this feature complements.
3. Read `backend/crates/core/src/archetype/mod.rs` to understand the
   `FrameFeatures` struct that drives archetype ranking.
4. Draft `requirements/0030-body-type-picker.md`.

**Context:** The current matching flow requires a photo upload and server-side
pose estimation. This is powerful but high friction — some users won't want to
upload a photo, or are onboarding before their first workout. The owner wants an
alternative: show 10–12 reference body silhouettes covering the main morphology
bands (ectomorph/mesomorph/endomorph × lean/moderate/bulky), let the user pick
the closest one, and derive approximate `FrameFeatures` from that selection —
skipping photo upload entirely. The photo route remains the primary (more
accurate) path; the image picker is the fallback.

**Acceptance criteria to include:**
- A "Don't want to upload a photo?" alternative path in the Flutter onboarding
- A screen showing 10–12 reference silhouette images (static assets, no API)
- User picks the closest silhouette + optionally adjusts an estimated body-fat %
  slider
- Selection maps to a synthetic `FrameFeatures` object that the existing
  `rank()` function can consume (same backend `POST /photo-sessions/:id/match`
  flow, OR a new lightweight endpoint that accepts synthetic features directly)
- The result feeds the same proposals → program flow (R-0014) as photo upload
- No photo stored when using the picker path (privacy)
- The picker is also available when re-matching (user can update their archetype
  without uploading a new photo)

**Record open questions** around: where the silhouette images live (bundled vs
CDN), how synthetic FrameFeatures are derived from a discrete selection (lookup
table vs formula), whether a new backend endpoint is needed or whether the
existing match endpoint can accept synthetic features directly.

Mark status **Accepted**.
Branch: `R-0030-body-type-picker` from `main`.
Commit: `R-0030: step-1 requirement — visual body-type picker (Accepted)`

---

## TASK 4 — Draft Requirement R-0031: Nutrition LLM Substitution

**Branch:** create `R-0031-nutrition-substitution` from `main`
**Output:** `requirements/0031-nutrition-substitution.md`

### Prompt for agent

You are writing a new requirement file for the fitAI project.

**Project:** Rust backend + Flutter mobile.
Working directory: `/Users/goose/projects/fitAI`.

**What to do:**
1. Read `requirements/0014-program-diet-from-archetype.md` for format.
2. Read `backend/crates/api/src/lib.rs` to see existing routes.
3. Read `backend/crates/core/src/program/mod.rs` to understand the
   `GeneratedDiet` structure (approach, macro emphasis, protein/carbs/fat/kcal).
4. Draft `requirements/0031-nutrition-substitution.md`.

**Context:** The system generates a diet plan (macros + approach) for the user.
The owner wants a feature where the user can ask "I don't have [food X], what
can I use instead?" and the system responds with a macro-equivalent substitute.
This is a Claude API call on the backend — the user's question + their active
diet plan → Claude answers with a substitute and the macro math.

**Acceptance criteria to include:**
- A new backend endpoint `POST /nutrition/substitute` (authenticated) accepting
  `{ food: string, quantity_g: number }` and returning
  `{ substitutes: [{ food, quantity_g, protein_g, carbs_g, fat_g, kcal, note }] }`
- The endpoint calls the Claude API (claude-haiku-4-5 for cost) with the user's
  active diet plan macros as context
- The Flutter UI surfaces this as a "Can't find it?" button on the diet plan
  screen, opening a simple text input + results list
- Response is never cached beyond the request (LLM answers are contextual)
- Error handling: if Claude API is unavailable, return 503 with a retryable flag
- The Rust backend stores the Anthropic API key as an environment variable
  (never hardcoded)
- Rate-limit: max 10 substitution calls per user per day (simple counter in
  Postgres) to control LLM costs
- Tests: unit test for the prompt-construction logic; integration test stubs the
  Claude client and asserts the response shape

**Record as decided:** Use `claude-haiku-4-5-20251001` for cost efficiency.
**Defer to spec:** exact prompt engineering, Rust HTTP client choice for the
Anthropic API (reqwest vs anthropic-sdk-rust if available), whether to use
streaming.

Mark status **Accepted**.
Branch: `R-0031-nutrition-substitution` from `main`.
Commit: `R-0031: step-1 requirement — nutrition LLM substitution (Accepted)`

---

## TASK 5 — Research: Cloud Model Training Architecture

**Branch:** none (deliver as a markdown research note, no code)
**Output:** paste back or write to `docs/ml-cloud-training-research.md`

### Prompt for agent

You are a research agent for the fitAI project — a Rust backend + Flutter mobile
fitness app. Do NOT write any code. Produce a technical research note.

**Context:** The fitAI roadmap includes an adaptive ML model (R-0015/R-0016,
Milestone M5) that learns per-user physiological response from workout logs and
adjusts training/nutrition recommendations over time. The stack is:
- Phase 1: `linfa` (Rust ML crate) — regression and tree models on structured
  tabular logs
- Phase 2: `burn` or `tch-rs` — sequential/time-series models
- Infrastructure: Rust backend on Docker (AWS or Azure cloud target, see
  `project-specifics.md`)

**Research questions — answer each concisely with sources:**

1. **linfa in production:** Can `linfa` models be trained incrementally (online
   learning) or only in batch? What is the realistic latency for re-training a
   regression model on ~10k rows? Can the trained model artifact be serialized
   and hot-reloaded without a server restart?

2. **burn vs tch-rs for Phase 2:** Given the server is CPU-only for MVP (no GPU
   budget), which handles CPU inference better? Which has better ONNX export
   support (we already use ONNX Runtime for pose estimation)?

3. **Cloud training options:** For a small Rust shop, what's the lightest path
   to scheduled model retraining in the cloud? Options to evaluate:
   - A nightly Rust binary (`cargo run --bin retrain`) triggered by a cron job
     on the same server
   - AWS SageMaker (overkill?)
   - A separate Fargate task / Lambda triggered by CloudWatch Events
   - Fly.io machines spun up on demand
   Assess each for: cost at <1000 users, operational complexity, fit with Rust.

4. **Model versioning:** How should the trained model artifact be stored and
   versioned so the API server can load a new model without downtime? (Blue-green
   model swap, S3 artifact store, etc.)

5. **Privacy:** Users' workout logs are training data. What's the minimum
   required for GDPR/LATAM compliance — anonymization approach, data retention
   policy, user right-to-delete implications for the training set?

Produce a structured markdown note (500–800 words) with a recommendation
section at the end.

---

## TASK 6 — Research: flutter_tts + audio_service Integration Patterns

**Branch:** none (paste back as a technical note)
**Output:** summary note for the main session to use in SPEC-0027 review

### Prompt for agent

You are a research agent for the fitAI project. Do NOT write any production
code. Produce a technical integration note.

**Context:** R-0027 (earbud-guided training) will use `flutter_tts` for
text-to-speech and `audio_service` for background audio + media-button handling.
The main session needs to review SPEC-0027 before implementation. Help by
answering these questions so the spec can be validated:

1. **flutter_tts + audio_service coexistence:** Do these two packages conflict
   over `AVAudioSession` on iOS? Is there a known setup order (initialize
   `audio_service` first, then `flutter_tts`)?

2. **audio_service minimal setup:** What is the minimum `AudioHandler`
   implementation needed to:
   (a) keep a foreground service alive on Android while TTS plays
   (b) register for the iOS `MPRemoteCommandCenter` play/pause button
   without actually streaming audio from `audio_service` itself (TTS does the
   audio — `audio_service` is only for the button + foreground service)?

3. **Media button on iOS with phone locked:** Does `MPRemoteCommandCenter`
   respond to the earbud button when the screen is locked and no media is
   "playing" in the traditional sense? Is there a trick (e.g. play a 0-second
   silent track) to keep the command center active?

4. **flutter_tts background on iOS:** Does `flutter_tts` require the
   `audio` background mode in `Info.plist`? What is the exact key and value?

5. **Known issues / gotchas:** List any known pub.dev issues, Flutter version
   incompatibilities, or Android 14+ permission changes that affect either
   package as of mid-2026.

Produce a concise technical note (300–500 words). Flag anything that might
require a spec change or a package substitution.

---

## Summary Table

| Task | Output | Blocks | Urgency |
|------|--------|--------|---------|
| 1 — SPEC-0027 | `specs/0027-earbud-guided-training.md` | R-0027 Step 2 → main session needs for Step 3 | **Now** |
| 2 — R-0029 Web req | `requirements/0029-web-frontend.md` | SPEC-0029 (later) | High |
| 3 — R-0030 Body picker req | `requirements/0030-body-type-picker.md` | SPEC-0030 (later) | High |
| 4 — R-0031 Nutrition LLM req | `requirements/0031-nutrition-substitution.md` | SPEC-0031 (later) | High |
| 5 — Cloud ML research | `docs/ml-cloud-training-research.md` | R-0015/R-0016 spec | Medium |
| 6 — flutter_tts research | note back to main session | SPEC-0027 review | **Now** |

Start Task 1 and Task 6 first — they directly unblock the main session.
Tasks 2–4 can run simultaneously with each other and with 1 + 6.
Task 5 is background research, lowest time pressure.
