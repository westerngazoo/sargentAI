# SPEC-0012 — Archetype library

- **Status:** Accepted
- **Realizes:** R-0012
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-10
- **Depends on:** SPEC-0002 (Implemented) — `AppState`, `AuthenticatedUser`, `ApiError`, the router; SPEC-0003 (Implemented) — reuses `core::Goal` for `goals_served`.
- **Module(s):** `backend/crates/core/archetype` (new — the pure domain + the embedded curated library), `backend/crates/api/archetype` (new — the read HTTP surface + the wire DTO), `backend/crates/api/lib` (router merge). **No migration, no DB** (the archetypes are static reference data — see §2.3).

## 1. Motivation

Realizes [R-0012](../requirements/0012-archetype-library.md): the curated
archetype library — the matching **prior**. Its novelty is that the data *is* the
deliverable: six structured, provenance-flagged records (frame profile + program
+ diet) that R-0013 matches a user's photo-derived frame against and R-0014
instantiates a starting plan from. R-0012 builds the schema, the curated data,
and a read API — no matching, no generation, no ML.

## 2. Design

### 2.1 Shape

```
core::archetype
  Archetype { id (slug), internal_name, display_name, summary,
              frame_profile, program_template, diet_template, provenance,
              goals_served }
  library() -> &'static [Archetype]   // the six, validated once
api::archetype
  GET /archetypes        -> [ArchetypeResponse]   (authenticated)
  GET /archetypes/:id    -> ArchetypeResponse | 404
  ArchetypeResponse      // OMITS internal_name + provenance.sources (AC4)
```

### 2.2 The model (`core::archetype`, pure)

- **`Archetype`** (validated; built through `Archetype::new`, returning
  `ArchetypeError`):
  - `id: &'static str` — a stable kebab slug (`"heavy-duty-mass"`), the API key;
  - `internal_name: &'static str` — the research label (`"Yates-96"`), **never
    serialized to a user** (AC4);
  - `display_name`, `summary` — abstracted, user-facing;
  - `frame_profile`, `program_template`, `diet_template`, `provenance`;
  - `goals_served: Vec<Goal>` (reuses `core::Goal`; non-empty).
- **`FrameProfile`** — the matchable structure (AC6):
  - numeric: `shoulder_to_waist: f64` (V-taper proxy, validated `1.0..=2.5`),
    `height_cm_band: HeightBand`;
  - categorical (controlled enums): `clavicle_width: WidthBand`
    (`Narrow`/`Average`/`Wide`), `limb_length: LengthBand`
    (`Short`/`Average`/`Long`), `build: Somatotype` (`Ecto`/`Meso`/`Endo`);
  - `structure_tags: Vec<StructureTag>` — a **controlled enum** vocabulary
    (`WideClavicles`, `NarrowHips`, `BlockyWaist`, `LongLimbs`, `ShortLimbs`,
    `DenseMuscle`, …), not free strings (AC6).
- **`ProgramTemplate`** — `philosophy: TrainingPhilosophy` (enum: `Hit`,
  `HighVolumeSplit`, `Powerbuilding`, `ModernHypertrophy`, …), `split: String`,
  `weekly_frequency_per_muscle: u8` (validated `1..=7`), `volume: VolumeBand`
  (`Low`/`Moderate`/`High`), `intensity: String` (rep/effort scheme),
  `rest: String`, `progression: String`.
- **`DietTemplate`** — `approach: String`, `calorie_strategy: String`,
  `macro_emphasis: MacroEmphasis` (enum), `meal_structure: String`.
- **`Provenance`** — `confidence: Confidence` (`Documented`/`Reconstructed`/
  `Folklore`), `sources: Vec<&'static str>` (**internal-only**, never serialized).
- **`ArchetypeError`** with `.field()` (the workout/nutrition error idiom): names
  the offending field for a malformed record.

The numeric/enum choices are deliberately the **shape R-0013's pose estimation
emits** (a shoulder-to-waist ratio, banded clavicle width, somatotype) so
matching is a weighted nearest-neighbor over these fields.

### 2.3 Storage — embedded, not a DB table (key decision, OQ-H1)

The archetypes are **static curated reference data**, not user data: they change
only via a reviewed code change, and "the owner approves each record" maps
exactly to **reviewing the data in a PR diff**. So the library is **embedded in
the binary** as a typed `core::archetype::seed` module exposing
`library() -> &'static [Archetype]` (a `OnceLock`-initialized, validated `Vec`),
**not** a Postgres table + seed migration. Consequences:

- no migration, no `db.rs` change, no per-request DB round-trip;
- R-0013 reads the same in-memory `library()` for matching (no second path);
- a unit test asserts **every record validates** — so an invalid record can
  never ship (the build's test gate is the guard);
- evolving the schema is a code change reviewed with the data, not a data
  migration. (A DB-backed library is revisited only if archetypes ever become
  user-editable — a non-goal, R-0012 §4.)

### 2.4 Wire DTO — `internal_name` and `sources` never cross (AC4)

The `core::Archetype` is **not** `Serialize`. The HTTP layer owns an
`ArchetypeResponse` DTO (the R-0003 `ProfileResponse` precedent) that serializes
`id`, `display_name`, `summary`, `frame_profile`, `program_template`,
`diet_template`, `confidence` (the provenance level), and `goals_served` — and
**omits `internal_name` and `provenance.sources`** (likeness/legal + internal
research notes). A test pins that the JSON carries no `internal_name`/`sources`
key.

### 2.5 Read API (`api::archetype`)

- `GET /archetypes` (authenticated) → `200` + `[ArchetypeResponse]` for the whole
  library (stable order: as authored).
- `GET /archetypes/:id` → `200` the match / `404` an unknown slug.
- Authenticated (the `AuthenticatedUser` extractor) — consistent with the
  app being auth-gated; archetypes are not anonymous content (OQ-H4). `401`
  unauthenticated.

### 2.6 The prior-only guardrail (AC5)

The library lives in `core::archetype` with a module-level doc stating it is the
**matching prior** and must **never** be read as training data by the M5
response model. There is no code path from `archetype` into any training input;
M5 requirements (R-0015/16/17) consume *user logs*, not this module. The
guardrail is documentation + module boundary (a lint/test can later assert no
M5 module imports `archetype::seed`).

## 3. Code outline

```rust
// core/src/archetype/mod.rs (excerpt)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Confidence { Documented, Reconstructed, Folklore }

pub struct FrameProfile { /* shoulder_to_waist, height_band, clavicle_width,
                             limb_length, build, structure_tags */ }

pub struct Archetype { /* …§2.2… */ }
impl Archetype {
    /// # Errors
    /// [`ArchetypeError`] for an out-of-range ratio/frequency, an empty
    /// goals list, or an empty name.
    pub fn new(/* fields */) -> Result<Self, ArchetypeError> { /* validate */ }
}

/// The curated library — the six, validated once at first access.
pub fn library() -> &'static [Archetype] {
    static LIB: OnceLock<Vec<Archetype>> = OnceLock::new();
    LIB.get_or_init(seed::all)
}

pub fn find(id: &str) -> Option<&'static Archetype> {
    library().iter().find(|a| a.id == id)
}

// seed/mod.rs — the single justified `expect` (architect finding 1, option B).
#[allow(clippy::expect_used)] // seed records are compile-time constants proven
                              // valid by `archetype_seed_all_records_validate`
                              // (SAC2); an invalid record fails the build, so
                              // this is a genuinely unreachable state (CLAUDE.md §6).
pub(crate) fn all() -> Vec<Archetype> {
    vec![
        Archetype::new(/* heavy-duty-mass / Yates-96 */)
            .expect("seed record must validate (SAC2)"),
        // … the other five …
    ]
}
```

```rust
// api/src/archetype/handlers.rs (excerpt)
pub(crate) async fn list(_user: AuthenticatedUser) -> Json<Vec<ArchetypeResponse>> {
    Json(fitai_core::archetype::library().iter().map(ArchetypeResponse::from).collect())
}
pub(crate) async fn get_one(_user: AuthenticatedUser, Path(id): Path<String>)
    -> ApiResult<Json<ArchetypeResponse>> {
    fitai_core::archetype::find(&id).map(|a| Json(ArchetypeResponse::from(a)))
        .ok_or(ApiError::NotFound)
}
```

> **Validity contract (architect finding 1, resolved → option B).** `library()`
> stays **infallible** (`&'static [Archetype]`), so `find`/`list`/`get_one` keep
> their drafted signatures. `seed::all` discharges each `Archetype::new(...)
> Result` with **one justified `expect`** carrying `#[allow(clippy::expect_used)]`
> and a message — permitted because the records are compile-time constants and
> the SAC2 test (`archetype_seed_all_records_validate`) fails the build on any
> malformed record, making the panic genuinely unreachable (CLAUDE.md §6). The
> alternative (a `Result`-returning `library()` threading a never-triggered
> 500 through every reader) was rejected as noise for proven-valid data.

## 4. Non-goals

Inherits R-0012 §4: no matching (R-0013), no program/diet generation (R-0014), no
ML/training, no mobile UI, no user-editable archetypes, no medical claims, no
athlete imagery. Also: no DB table, no admin write API, no per-record versioning.

## 5. Open questions (for the architect review)

- **OQ-H1 — Embedded library vs DB table?** Proposed: **embedded** typed Rust
  (§2.3) — static curated data, reviewed-in-PR, no migration; R-0013 reads it
  in-memory. DB only if archetypes ever become user-editable (a non-goal).
- **OQ-H2 — `seed::all` validity contract. RESOLVED (architect finding 1) →
  option B:** `library()` stays infallible; `seed::all` uses one justified
  `expect` with `#[allow(clippy::expect_used)]` + a message tied to the SAC2
  validity test. A `Result`-returning `library()` was rejected (a never-triggered
  500 path for compile-constant data). See §3.
- **OQ-H3 — `structure_tags` as a controlled enum vs validated strings.**
  Proposed: a `StructureTag` enum (controlled vocab, AC6) over free strings.
- **OQ-H4 — Read API authenticated vs public?** Proposed: **authenticated** —
  consistent with the auth-gated app.
- **OQ-H5 — `id` as a stable slug vs UUID?** Proposed: a kebab **slug**
  (`heavy-duty-mass`) — readable, stable, and the right key for curated singletons.

## 6. Acceptance criteria

- [ ] **SAC1 → AC1.** The `Archetype` model + value types validate via
  `Archetype::new`; an out-of-range ratio/frequency, empty goals, or empty name
  is rejected with the right `field()`.
- [ ] **SAC2 → AC2/AC7.** `library()` returns the six; a unit test asserts **every
  record validates** and that each `provenance.confidence` is set honestly
  (documented vs reconstructed vs folklore, per the curated sources).
- [ ] **SAC3 → AC3.** `GET /archetypes` → `200` + six records (authenticated);
  `GET /archetypes/:id` → `200` a known slug / `404` unknown; `401` without a
  token.
- [ ] **SAC4 → AC4.** The `ArchetypeResponse` JSON carries `display_name` but
  **no `internal_name` and no `sources`** key (asserted on the serialized body).
- [ ] **SAC5 → AC5.** The library lives in `core::archetype` with the prior-only
  doc; no `archetype` symbol is imported by any training path (none exists yet —
  the boundary is documented for M5).
- [ ] **SAC6 → AC6.** `frame_profile` exposes the numeric ratio + banded/enum
  fields R-0013 will match on; `structure_tags` is the controlled `StructureTag`
  set.
- [ ] **SAC7 → AC8.** Unit (`core::archetype`) + integration (`/archetypes`)
  suites; `cargo fmt`/`clippy`/`test`/`build` green.
- [ ] **SAC8 → AC9.** No matching/generation/ML/DB/mobile in the diff — the
  library + read API only.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **Embedded typed-Rust library (`core::archetype::seed`), no DB table/migration.** | Static curated reference data; "owner approves each record" = PR review; R-0013 reads it in-memory; no per-request DB cost. (OQ-H1) |
| 2026-06-10 | **`core::Archetype` not `Serialize`; an `ArchetypeResponse` DTO omits `internal_name` + `sources`.** | Likeness/legal: famous names + research sources never cross the wire (the R-0003 derived-DTO precedent). (AC4) |
| 2026-06-10 | **Structured `FrameProfile`: numeric ratio + banded/enum fields + a controlled `StructureTag` vocab.** | The shape R-0013 matches against; controlled vocab keeps matching well-defined. (AC6/OQ-H3) |
| 2026-06-10 | **Stable kebab-slug `id`; authenticated read API.** | Readable curated-singleton keys; consistent with the auth-gated app. (OQ-H4/H5) |
| 2026-06-10 | **Validity is test-gated, not a runtime error path.** | The data is a compile-constant authored to validate; a unit test fails the build on any malformed record, so `library()` need not surface a runtime error. (OQ-H2) |
| 2026-06-10 | **Prior-only guardrail via module boundary + docs.** | The famous data is the matching prior; it must never feed the M5 response model — enforced by there being no code path from `archetype` to training. (AC5) |
| 2026-06-10 | **(architect finding 1) `library()` infallible; `seed::all` uses one justified `expect` (`#[allow(clippy::expect_used)]` + message), not a `Result` surface.** | Records are compile-constants test-gated by SAC2; the panic is genuinely unreachable (§6 allowance). Avoids a never-triggered 500 path through every reader. |
| 2026-06-10 | **(architect) `structure_tags` is a controlled `StructureTag` enum — deliberately STRONGER than AC1's "free `structure_tags`" wording.** | Controlled vocab makes R-0013 matching well-defined (the `Angle`/`Goal` precedent); noted so qa reads it as an intentional tightening, not a miss. (OQ-H3) |

## Changelog

- _2026-06-10 — created (Draft). Realizes the accepted R-0012. Five HOW-level design questions (OQ-H1..H5) raised for the architect review; the central call is embedded-typed-Rust over a DB table. The six curated records are presented to the owner for approval before step-5 implementation._
- _2026-06-10 — **Accepted.** Architect review returned APPROVE WITH NITS; all five OQ-H approved. Finding 1 applied: `library()` stays infallible with a single justified `expect` in `seed::all` (option B), keeping `find`/`list`/`get_one` signatures; `structure_tags` controlled-enum tightening recorded as deliberate. **Owner approved all six curated records** (Yates/Mentzer/Arnold/Columbu/Cutler/Heath — content, frame priors, training/diet templates, provenance flags) for the seed._
