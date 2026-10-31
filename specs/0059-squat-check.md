# SPEC-0059 — The squat check: the range endpoint and the physics check widget

- **Status:** Accepted — owner, 2026-10-27, together with R-0059 (OQ-A: `400`)
- **Realizes:** R-0059 (Accepted 2026-10-27)
- **Author:** Claude (main session), for owner review
- **Created:** 2026-10-25
- **Depends on:** SPEC-0045 (slices A and B1: `squat_range`), SPEC-0061 (§2.5 the day region, §2.10 the look, §2.12 slice 4)
- **Module(s):** `backend/crates/api/src/biomech/` (new), `backend/crates/core/src/plan.rs` (`PlanExercise`), `backend/crates/core/src/biomech/model.rs` (`model_for`), `mobile/lib/src/physics/` (new), `mobile/lib/src/home/` (the Física button)

## 1. Motivation

R-0059: the lifter sees their own squat as an animation, computed on the
server from their measurements and load. R-0061 AC9 then places that check
**inside the selected day**, one per exercise, as the product's selling point
(design v2, `docs/design/one-screen-v2.html`). This spec is the server half
(an endpoint over SPEC-0045's `squat_range`) and the widget half (the check
in the reels' grammar), and it is what R-0061 slice 4 consumes.

Two facts shape it:

1. **The engine already exists and is pure.** `fitai_core::biomech::squat_range(body, bar, ankle, frames)`
   returns frames with postures and four moments, and either reaches the
   bottom or says why it stopped (SPEC-0045 §2.6.6). The endpoint adds
   validation, the lever arithmetic the check shows, a wire envelope, and
   nothing else.
2. **"COMPROBABLE" is a promise about arithmetic.** The reels' house rule is
   that a label and a panel that disagree is a bug, and that every number
   comes from the computation on screen. So the sum the check shows,
   `F × d + body = τ`, is computed **on the server from the same model**, and
   the client only formats it. A client-side residual would make the
   promise hollow (architect finding 3).

## 2. Design

### 2.1 No engine seam, yet

The handler calls `fitai_core::biomech::squat_range` directly. The pure
function is the seam: when the garust kernel lands (owner's hard requirement
2026-10-23, R-0045 amendment #160), its interface is not known today, so a
trait cut now would be re-cut then. One call site changes. (Architect
finding 2: an interface with one implementation, and an unknown second, is
debt.)

### 2.2 `POST /biomech/squat/range` (AC1, AC2, AC3)

Authenticated (`AuthenticatedUser`), stateless, logs nothing from the body
(AC8).

**Request:**

```json
{
  "height_m": 1.78, "mass_kg": 77.0,
  "thigh_m": 0.45, "shank_m": 0.42, "trunk_m": null,
  "dorsiflexion_deg": 29.0,
  "bar_kg": 84.0,
  "frames": 41
}
```

| Field | Rule | On violation |
|---|---|---|
| `height_m` | `0.5..=3.0`, the profile's own range (`core::profile`, `height_cm / 100`) | `400 validation`, `field: "height_m"` |
| `mass_kg` | `20.0..=500.0`, the profile's own range | `field: "mass_kg"` |
| `thigh_m`, `shank_m`, `trunk_m` | optional; each `0.1..=1.5` | that field |
| `dorsiflexion_deg` | required, `0.0..=90.0` (`Dorsiflexion::from_degrees`) | `field: "dorsiflexion_deg"` |
| `bar_kg` | optional; `LoadKg::try_new` (`(0, 1000]`) | `field: "bar_kg"` |
| `frames` | optional, default 41; `FrameCount::new` (`2..=241`) | `field: "frames"` for an integer out of range; a non-integer is a malformed body (`field: "body"`), as everywhere in the API |

`scale` and `MeasuredLengths` validate nothing (`anthro.rs`), so these are
the only guards. JSON carries no NaN or infinity, so finiteness is free. The
height and mass ranges are the profile's, so the app can never send a value
the profile accepted and get a 400 for it (finding 6). The app sends the
profile's `height_cm / 100`.

**Status for an invalid field.** R-0059 AC1 says `422`. Every other endpoint
answers an invalid field with `400 {"error":"validation","field":…}`
(`error.rs`, SPEC-0002), the only validation shape the API has; its `422
Unprocessable { reason }` carries a fixed token and cannot name a field,
and `ApiException.fromDio` reads `field` flat. This spec uses `400
validation` and asks for a one-word amendment of AC1 (§5, owner may veto).

**Response**, always `200` for a well-formed request. The envelope wraps
core's `Serialize` output **unchanged**; the example below is a real dump
(two frames, abbreviated), so the names are the ones the wire carries:

```json
{
  "end": { "kind": "bottom" },
  "table_lengths": ["trunk"],
  "frames": [
    {
      "progress": 0.0,
      "shank_deg": 3.40, "knee_flexion_deg": 0.0, "hip_flexion_deg": 0.0, "trunk_lean_deg": 3.40,
      "posture": {
        "ankle": {"x": -0.070, "y": 0.069}, "knee": {"x": -0.045, "y": 0.489},
        "hip": {"x": -0.019, "y": 0.938}, "shoulder": {"x": 0.012, "y": 1.450},
        "load": {"kg": 84.0, "at": {"x": 0.010, "y": 1.424}}
      },
      "mechanics": {
        "joints": [
          {"joint": "hip",    "resisting": "hip_extensors",   "moment": 33.52,  "uncertainty": 0.63},
          {"joint": "knee",   "resisting": "knee_extensors",  "moment": -71.49, "uncertainty": 0.63},
          {"joint": "ankle",  "resisting": "plantar_flexors", "moment": 109.56, "uncertainty": 0.63},
          {"joint": "lumbar", "resisting": "spinal_erectors", "moment": 22.74,  "uncertainty": 2.05}
        ],
        "load": {"basis": "bar", "kg": 84.0},
        "assumptions": [{"assumption": "sagittal_plane"}, {"assumption": "single_posture", "at": "upright"}, "…"],
        "uncertainty": {"l5s1_placement": 0.03, "com_placement": 0.02}
      },
      "hip": {"lever_m": 0.029, "bar_n": 824.04, "bar_nm": 23.82, "body_nm": 9.70},
      "ground_n": 1579.41
    }
  ],
  "assumptions": [{"assumption": "joints_advance_together"}, {"assumption": "straight_body_upright"}]
}
```

- `end` is an **api-owned** enum, `#[serde(tag = "kind", rename_all = "snake_case")]`:
  `Bottom | Stopped { reached, bottom: Option<Rejection> } | Rejected { rejection: Rejection }`,
  with `impl From<RangeEnd>` for the first two. `Rejected` is this endpoint's
  answer to `squat_range`'s `Err(Rejection)`: `frames: []` and the typed
  reason (AC3), never an empty animation without a reason; it carries no
  range `assumptions` either (`[]`), since core's `Err` states none. It is not added
  to core's `RangeEnd`, whose promise is "the largest balanced prefix"
  (finding 5). `Rejection` serializes as core does (`reason`, `limiting`,
  `needed_deg`, `available_deg`, …).
- `frames[]` is core's `RangeFrame` (`progress`, `shank_deg`, the three
  angles, `posture`, `mechanics`) **plus two fields the check needs** and
  the client must not compute:
  - `hip.lever_m`: the signed horizontal distance from the hip to the bar's
    line, bar minus hip, positive with the bar ahead (the design's lever;
    bodyweight-only frames carry no `hip`);
  - `hip.bar_n = bar_kg × g`, `hip.bar_nm = bar_n × lever_m`;
  - `hip.body_nm = body_moment_about(Joint::Hip, &body, &posture)`
    (`chain.rs`), the segments' share **in the loaded posture shown**, under
    the stated placement conventions. Not the posture with the load removed:
    core places the arms' centre of mass at the shoulder → bar midpoint with
    a bar and hanging without one, so removing the load would also move the
    arms and the shares would stop summing to the hip moment;
  - `ground_n = (bar_kg + mass_kg) × g`.

  A Rust test asserts, per frame, `bar_nm + body_nm == joints[0].moment` to
  `1e-9`. That is the whole "La cuenta" line, and it is the server's
  arithmetic. The envelope's `g` is core's `G` (`9.81`), the one constant
  every moment is multiplied by (SPEC-0045 §2.2); moments are multiplied by
  it only in `chain.rs`, and the API multiplies it only for the displayed
  weights `bar_n` and `ground_n`.
- Range-level `assumptions` are `SquatRange.assumptions` (the convention
  ones); per-frame assumptions sit inside `mechanics`, as core emits them.
- `table_lengths` lists which of `thigh`, `shank`, `trunk` came from the
  table because the request left them null (AC2), computed from the request.
- **One golden test** pins this JSON (a two-frame fixture, compared as
  parsed values with a 1e-6 tolerance on floats), so a core refactor cannot
  change the wire unnoticed; core's own `tests/range.rs` already pins `end`,
  `bottom.reason` and the posture metres.

**Privacy (AC8).** `RangeRequest` has a hand-written `Debug` that prints
only which optional fields are present. There is no `TraceLayer` in
`lib.rs` (tower-http `cors` only), so the `#[traced_test]` is a guard, not a
reproduction: it exercises the **three** paths (valid, `JsonRejection`,
engine `Rejection`) and asserts none of the body's numbers reach the capture
(finding 8). R-0024's inventory gains "squat check request: processed, not
stored".

### 2.3 The client model (mobile)

`lib/src/physics/`:

- `data/squat_check_api.dart`: `Future<SquatRangeResult> range(SquatRangeRequest)`.
- `domain/squat_range.dart`: the wire shapes above, parsed once; `End` as a
  sealed class (`Bottom`, `Stopped(reached, bottom?)`, `Rejected(reason)`).
- `domain/lever_text.dart`, **formatting only**: "824 N × 0.03 m = 23.8 N·m,
  + 9.7 del cuerpo = 33.5 N·m" from the frame's `hip` fields and the
  server's hip moment, with a **decimal point**: Spanish numbers follow the
  `es_MX` convention (one helper, `numberLocale`; the catalogue stays `es`),
  since a decimal comma collides with the line's own commas. No physics on
  the client (AC7); the test checks the string, not a sum.
- `application/squat_check_controller.dart`: holds the request inputs
  (measurements, dorsiflexion, the chosen load), calls the API, exposes
  `AsyncValue<SquatRangeResult>`, and interpolates between returned frames
  for playback (display only, AC7).

**Measurements in the request (AC2).** Height and mass come from the profile
(R-0003), `height_cm / 100`. Limb lengths come from R-0058 once it exists;
until then they are null and the result says `table_lengths`. **Dorsiflexion
has no default** (SPEC-0045 §2.6.1.1): the first time a lifter opens the
check it asks for the degrees in one field with a link to the measuring
guide, and remembers the value for the session (`StateProvider`), not on
the server, until R-0058 stores it.

**Loads (OQ-2).** Bodyweight, plus the load typed for that exercise's sets
when the day has one, plus a field to type another. v1 does not read logged
sets.

### 2.4 The widget: `SquatCheck` (AC4, AC5, AC6)

Design v2's physics card, in SPEC-0061 §2.10's tokens and faces:

- **Figure** (`CustomPainter`): feet, shank, thigh, trunk, head, hanging
  arms, the bar with a plate to scale; the mid-foot balance line dashed; the
  hip lever arm dashed from the hip to the bar's line; the bar and ground
  **force vectors to one scale** (N per logical pixel chosen from the
  largest `ground_n` in the result), coloured by the tension ramp.
- **Readouts** in `panelito` boxes: τ cadera (N·m), palanca cadera (cm,
  from `hip.lever_m`), τ rodilla, β tronco; knee and hip flexion and
  progress in the mono face.
- **Moment bars**, one per group, in the tension ramp; a **negative** moment
  draws to the left of the zero line in the neutral grey (AC4).
- **Live graph** of τ cadera and τ rodilla against bar height (`posture.load.at.y`),
  with the current point moving.
- **Playback**: play/pause and a scrubber over progress; the controller
  interpolates frames.
- **La cuenta**: the server's `hip` fields, formatted.
- **The caveats** (AC5), always visible under the figure: the assumptions
  (range-level and the current frame's), which lengths came from the table,
  how the model places the arms ("sin barra, los brazos cuelgan; con barra, el modelo los pone entre el hombro y la barra"), and "entre
  de pie y el fondo las piernas siguen una convención del modelo; tu curva
  real saldrá de tu video".
- **A stopped range** says, in plain words, how far the lifter gets and why
  (`Stopped`: "sin carga tu tobillo te detiene al 57 % de la bajada");
  `Rejected` shows the typed reason and no figure (AC3).
- **No injury or safety claim** (AC6): heat means tension magnitude; no copy
  says safe or dangerous; a word-list test over the catalogue entries used
  by the widget enforces it.
- **The stamp** reads **COMPROBABLE**, never COMPROBADO (SPEC-0061 §2.10).
- Every string is a catalogue entry (R-0061 AC11).

### 2.5 Where it lives (R-0061 AC9, slice 4)

In the selected-day region, each exercise shows a **Física** button. Which
exercises have a model is **the server's call, not a client string list**
(finding 7). A training day's exercises are objects on the schedule's
`cycle`, `{"name": "Back Squat", "physics": "squat"}`, so a name and its
physics cannot disagree: `core::plan::PlanExercise` is built from the name
alone, and `physics` (`Lift::Squat` today, or null) comes from one function
in `core`, `biomech::model_for(name)`. It matches exactly after trim and
lowercase, never fuzzily: "Back Squat", "Sentadilla" and "Sentadilla
trasera" are the squat; a bare "Squat", "Sentadilla frontal", a front, hack
or low-bar squat are not. A free session's exercises get the same lookup
through the names the app already sends (59-c2). For
`physics: null` the button reads **Pronto** and opens nothing. On a wide
window the check opens beside the exercise list; on a phone, below the
exercise. The deadlift and bench stay *Pronto* until R-0045's garust work
gives them a model.

### 2.6 Slicing

| Slice | Content | Stack |
|---|---|---|
| 59-a | `biomech` module: validation, the endpoint, the lever fields, the `End` enum, the golden and no-log tests | backend |
| 59-b | `physics` folder: API, models, `lever_text`, controller, with tests; `{name, physics}` exercises on `PlanDay` and the schedule (`biomech::model_for`) | both |
| 59-c1 (= R-0061 slice 4, first half) | the figure painter with vectors and lever, playback, the Stopped and Rejected states | mobile |
| 59-c2 (second half) | readouts, moment bars, the live graph, caveats, the Física/Pronto buttons, the free-session `model_for` lookup, the two layouts, the word-list test | mobile |

Each is within 100–400 lines (finding 4).

## 3. Code outline

```rust
// crates/api/src/biomech/handlers.rs
pub(crate) async fn squat_range(
    _user: AuthenticatedUser,
    req: Result<Json<RangeRequest>, JsonRejection>,
) -> ApiResult<Json<RangeResponse>> {
    let req = body(req)?;                                   // 400 validation { field }
    let (body, table_lengths) = req.body();                 // scale(height, mass, &measured)
    let response = match biomech::squat_range(&body, req.bar, req.ankle, req.frames) {
        Ok(range) => RangeResponse::from_range(&body, req.bar, range, table_lengths),
        Err(rejection) => RangeResponse::rejected(rejection, table_lengths),
    };
    Ok(Json(response))
}

// crates/api/src/biomech/lever.rs — pure, tested against the frame's hip moment
pub(crate) fn hip_lever(body: &Body, bar: LoadKg, frame: &RangeFrame) -> HipLever { … }
```

```dart
// mobile/lib/src/physics/domain/lever_text.dart — formatting only
String laCuenta(HipLever h, AppLocalizations l10n, String locale) { … }
```

## 4. Non-goals

- Saving a check or a variant (SPEC-0045 slice C).
- Low-bar, deadlift, bench: *Pronto* until modelled.
- Reading logged sets for the load list.
- Storing measurements or dorsiflexion (R-0058).
- Video comparison.
- Any physics on the client.

## 5. Open questions

- **OQ-A. R-0059 AC1's status code.** This spec uses the API's existing
  `400 validation { field }` instead of AC1's `422`. If the owner agrees, AC1
  is amended by one word; if not, a `422` variant with a field is added to
  `error.rs` for this endpoint alone. *Answered 2026-10-27:* `400`; AC1
  amended.
- **OQ-B. R-0059's acceptance.** R-0059 is still Draft. Its three open
  questions are answered by design v2 (the check lives inside the day; loads
  typed in v1; language is R-0061's catalogue). The owner accepts R-0059 and
  this spec together, or says what to change. *Answered 2026-10-27:* both
  accepted.

## 6. Acceptance criteria

| R-0059 AC | Verified by |
|---|---|
| AC1 | API tests: a valid request returns frames with the four moments, the `hip` and `ground_n` fields, `end`, both assumption lists; `bar_kg: null` → `load.basis: "bodyweight_only"` and no `hip`; each field at both bounds accepted and just outside → 400 naming it; no session → 401. One golden test pins the wire. |
| AC2 | API test: nulls in the lengths → `table_lengths` lists them; a full set → empty list. |
| AC3 | API tests: a lifter who stops before parallel → 200 `kind: stopped` with `reached` and the bottom rejection; a lifter the engine cannot place → 200 `kind: rejected`, `frames: []`. Widget tests: the stopped message in plain words; the rejected reason shown. |
| AC4 | Widget tests: figure, balance line, lever, vectors present; play/pause and scrubbing change the frame; a negative moment draws left of zero. |
| AC5 | Widget test: assumptions, table lengths, the arms line and the convention line are visible without scrolling the card. |
| AC6 | Word-list test over the widget's catalogue entries. |
| AC7 | Rust: `bar_nm + body_nm == hip moment` per frame to 1e-9. Dart: `lever_text` formats the server's numbers; the controller interpolates only between returned frames. |
| AC8 | `#[traced_test]` over the three paths: the body's numbers never reach the log. R-0024 inventory row. |
| AC9 | The above; the e2e test belongs to the `qa` agent. |

## 7. Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-10-25 | No engine trait; the handler calls `squat_range` | The second implementation's interface is unknown until #160 lands; the pure function is the seam (architect finding 2). |
| 2026-10-25 | The envelope wraps core's `Serialize` output unchanged and adds `hip` and `ground_n`; one golden test pins the wire | No second model of the same data; the test keeps a core refactor from silently changing the API (finding 1). |
| 2026-10-25 | `End` is an api enum with `Rejected`; core's `RangeEnd` is untouched | `RangeEnd`'s promise is the balanced prefix; `squat_range` returns `Result` for a reason (finding 5). |
| 2026-10-25 | The lever, the bar's moment and the body's share are computed on the server | "COMPROBABLE" means the sum is the model's arithmetic, not a client residual (finding 3). |
| 2026-10-25 | Validation ranges are the profile's | The app never gets a 400 for a value the profile accepted (finding 6). |
| 2026-10-25 | Which exercise has a model is a `physics` field decided in core | A client string list drifts from the server's names (finding 7). |
| 2026-10-25 | `400 validation { field }` for an invalid field (OQ-A) | The API's one error shape; the client already parses it. |
| 2026-10-25 | Dorsiflexion asked once and kept in session memory until R-0058 | SPEC-0045 commits no default; storing it is R-0058's job. |
| 2026-10-27 | Accepted with R-0059; OQ-A answered `400` | Owner. |
| 2026-10-28 | `g` is core's `G` (9.81), not 9.80665 | The identity `bar_nm + body_nm == hip moment` holds only with the constant core multiplies every moment by; changing it is a SPEC-0045 question (owner). |
| 2026-10-28 | `body_nm` is the segments' share in the loaded posture, from core's new `body_moment_about` | The arms' placement depends on the bar, so the unloaded posture is a different posture; one source in `chain.rs`, no copy in the API (owner; architect review of #180). |
| 2026-10-30 | Exercises are `{name, physics}` objects on the cycle, not a per-day `physics` field or a parallel list | A day lists several exercises, each with its own Física/Pronto button; one object per exercise means a name and its physics cannot disagree (owner, review of #183). |
| 2026-10-30 | The lookup is `biomech::model_for`, exact after trim and lowercase: "Back Squat", "Sentadilla", "Sentadilla trasera" | It says which mechanics are modelled, narrower than `Lift::from_exercise_name` (which lift a video shows); Spanish names for what lifters type in free sessions (owner). |
| 2026-10-30 | "La cuenta" uses a decimal point: Spanish numbers follow `es_MX` | The target market is Mexico; a decimal comma collides with the line's own commas (owner). |

## 8. Changelog

- 2026-10-25 — created (Draft).
- 2026-10-25 — architect review applied (§9): the real wire shape; no
  engine trait; server-side lever arithmetic; `End` as an api enum; the
  profile's ranges; `physics` on `PlanDay`; 59-c split in two; the no-log
  test over three paths.
- 2026-10-27 — Accepted by the owner with R-0059; OQ-A and OQ-B answered.
- 2026-10-28 — Amended with slice 59-a (#180), owner-approved: `g` = core's `G`;
  `body_nm` in the loaded posture via `body_moment_about`; the example's hip
  numbers from the golden; `rejected` carries no range assumptions; the
  `frames` row; the arms caveat matches the model.
- 2026-10-30 — Amended with slice 59-b (#183), owner-approved: exercises as
  `{name, physics}` objects decided by `biomech::model_for` (with the
  Spanish names); the decimal point in "La cuenta"; 59-c2 carries the
  free-session lookup.

## 9. Architect review — ACCEPT WITH CHANGES (2026-10-25), applied

| # | Severity | Finding | Where it landed |
|---|---|---|---|
| 1 | Blocking | The response example contradicted the derives it claimed to reuse | §2.2 rewritten from a real dump |
| 2 | Major | `SquatEngine` leaked the wire type and had no known second implementation | §2.1 dropped |
| 3 | Major | The client-side "La cuenta" was a residual, so its test was vacuous | §2.2 `hip`/`ground_n` server-side; §2.3 formatting only; §6 AC7 |
| 4 | Major | Slice 59-c was over 800 lines | §2.6 split into c1 and c2 |
| 5 | Minor | `Rejected` on core's `RangeEnd` | §2.2 api-owned `End` |
| 6 | Minor | Ranges narrower than the profile's | §2.2 table |
| 7 | Minor | A client string list for modelled exercises | §2.5 `physics` field |
| 8 | Minor | The no-log test could pass vacuously; bodyweight and bounds untested | §2.2, §6 |
