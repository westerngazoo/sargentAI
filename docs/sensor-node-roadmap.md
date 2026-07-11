# fitAI Sensor Node — Roadmap (Thingy auto-logging)

**Status:** proposal / handoff doc for a dedicated agent. Not yet in the fitAI
requirement loop.
**Author:** drafted 2026-07-09. **Owner:** Gustavo Delgadillo.

This document is self-contained: an agent should be able to execute it without
prior conversation context.

---

## 1. Goal (one sentence)

A carry-around **Nordic Thingy:52** that **auto-logs strength sets** — an RFID
tap says *which exercise*, the onboard IMU counts *how many reps* — streamed over
BLE to the fitAI phone app, which writes the set to the existing workout log. No
camera, no Raspberry Pi, no internet on the node.

## 2. Guiding principle

**The Thingy senses; the phone thinks.** The node ships raw/lightly-processed
signals over BLE; the Flutter app does rep counting, set assembly, and logging.
This keeps firmware within the nRF52832's RAM budget and defers on-device ML to a
later Thingy:53 upgrade (see M6).

## 3. Where this sits in fitAI

Auto-logging is the **third input modality**, alongside:
- **Voice** — R-0032 (shipped) / R-0037 (proposed conversational)
- **Earbud button** — R-0035 (proposed rebuild)
- **Sensor (this)** — new

All three write the **same** workout log (R-0004, `POST /workouts`). The sensor
path needs **no backend change for v1** — it is a new *client* of an existing
endpoint. It also becomes the **highest-quality data source for the M5 learning
model** (R-0015/16/17): auto-captured reps + tempo that a human would never type.

## 4. Two repos, two halves

| Half | Repo | Stack |
|------|------|-------|
| **Firmware** — sensing + BLE | **`goose-steel`** (GooseSteel C++23 framework) | nRF52832 / Thingy:52, onboard 9-axis IMU, existing RFID reader, BLE GATT |
| **App integration** — BLE client + logging | **`fitAI`** (`mobile/`) | Flutter + a BLE plugin, existing `program`/`workout` services |

The **BLE data contract (§6) is the seam** — freeze it first so both halves
proceed in parallel.

## 5. Decisions already made (do not relitigate without owner)

1. **Phone is the brain (v1).** Node streams; phone counts/logs. → Thingy:52 is
   sufficient; on-device ML is out of scope for v1.
2. **RFID does classification; IMU does counting.** Tagging the station removes
   the hard "which exercise?" problem, leaving only the tractable "count reps."
3. **No camera on the node.** Pose/physique stays a separate feature (a fixed
   camera looking *at* the athlete). A body-worn cam can't see the body.
4. **BLE-only; phone is the gateway.** No WiFi on the :52. A standalone
   (phone-absent) node is a later, separate track (needs a WiFi-capable board).

## 6. Milestones

Each milestone is independently demoable. **T0's data contract gates everything
else** and unblocks parallel firmware/app work.

### T0 — Foundations & the BLE contract *(do first)*
- **Define the GATT service** — one custom service with characteristics:
  - `RfidTag` (notify): the scanned tag id.
  - `ImuStream` (notify): timestamped accel (+gyro) samples at a fixed rate,
    packed to fit one BLE notification (respect MTU; batch samples per packet).
  - `SetControl` (write/notify): start/stop-set signaling + node state.
  - Freeze the packet byte layouts in a shared spec both repos cite.
- **GooseSteel scaffold** for the Thingy:52 board (board `.cxxm`, build via the
  Docker toolchain, host-native GTest harness — GooseSteel's strength; write the
  packing/unpacking with host tests before touching hardware).
- **Wake/power model** decided: always-advertising vs wake-on-motion vs
  wake-on-RFID. Battery life target stated.
- **Deliverable:** a documented GATT spec + a firmware skeleton that advertises
  and emits a dummy `RfidTag` notification.

### T1 — RFID → "which exercise"
- Firmware: emit a `RfidTag` notification on scan (reuse the working RFID path).
- App: a **station registry** (`tag id → exercise name`), editable in-app;
  tapping a tag starts a "set in progress."
- **Demo:** tap a tag → app shows "Bench press — set started."

### T2 — IMU → rep counting (on the phone)
- Firmware: stream accel (+gyro) over `ImuStream` while a set is active; stop on
  set end (power saving).
- App: a Dart **peak-detection rep counter** over the accel stream, with
  per-exercise sensitivity. Start simple (magnitude peaks + refractory period);
  iterate.
- **Demo:** perform 10 reps → app counts ~10.

### T3 — Auto-log a set end-to-end
- App: on set end (second tap / rest-timeout / "out"), assemble
  `{exercise, reps, weight?, tempo}` and `POST` to the workout endpoint (reuse
  the existing workout service).
- **Weight entry** — the IMU can't know the load. v1: a one-tap confirm/edit of
  weight (default = last logged weight for that exercise), or encode weight on
  the tag. Keep it to a single interaction.
- **Demo:** full loop — tap → lift → the set appears in the Progress screen.

### T4 — Robustness & UX
- Rest detection + automatic set close; reject false positives (walking, racking
  the bar ≠ reps).
- BLE reconnect/dropout resilience; battery + charge state surfaced in-app.
- Multi-set session flow; a session-end gesture ("out"/long-press).

### T5 — Calibration & accuracy
- Per-exercise rep-signature tuning; **log labeled {IMU window → true rep count}**
  pairs. Accuracy target, e.g. **±1 rep on barbell compounds**.
- Note: this labeled corpus is **training data** for T6 and for the fitAI M5
  learning model — capture it deliberately.

### T6 — On-device intelligence *(stretch; Thingy:53 upgrade path)*
- Move rep counting (and optionally IMU-only exercise classification, removing
  the RFID dependency) **onto the node** via TinyML (e.g. Edge Impulse) — needs
  the nRF5340's RAM, i.e. a **Thingy:53**.
- Enables phone-absent logging. Ship model updates via GooseSteel's existing
  **signed WiFi OTA**.

## 7. Cross-cutting

- **Freeze the T0 contract early** — it is the only hard dependency between the
  two agents/repos.
- **Privacy:** motion data is personal. Keep the path node → phone → fitAI
  backend; no third parties. State a retention stance.
- **Testing:** firmware logic (packet packing, rep-signature math) in GooseSteel
  host-native GTests; app rep-counter in Flutter unit tests with recorded IMU
  fixtures (no hardware in CI).
- **fitAI-side tracking:** the app half should enter the fitAI requirement loop
  as its own `R-NNNN` (BLE client + sensor auto-log). The firmware half is
  tracked in `goose-steel`.

## 8. Open questions to resolve before T1

- **OQ-1 Node placement:** on-body (convenient) vs on-the-bar (cleaner rep
  signal). Changes the counting math and mounting.
- **OQ-2 Weight-entry model:** per-tag preset, per-set confirm, or last-value
  default.
- **OQ-3 Set boundaries:** what starts/ends a set — RFID double-tap, motion
  onset/rest-timeout, button, or a mix.
- **OQ-4 Station registry ownership:** who maps tags → exercises (user setup vs
  pre-provisioned tags).

## 9. First move for the assigned agent

Start at **T0**: write the GATT/data-contract spec and the GooseSteel board
scaffold with host-native tests for the packet codec. Everything else forks from
that interface. Resolve OQ-1..OQ-4 with the owner before T1.
