# Agent Task — Write SPEC-0027 (Earbud-guided training)

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** `R-0027-earbud-training` (already exists — checkout and push to it)
**Output file:** `specs/0027-earbud-guided-training.md`

---

## What you are doing

You are the spec-writer for the fitAI project. Your job is to produce the
technical spec (SPEC-0027) for the earbud-guided training feature. The
requirement is already accepted — your job is to say HOW it is built, not WHAT
it does.

---

## Step 1 — Read these files before writing anything

```
requirements/0027-earbud-guided-training.md   ← the accepted requirement (all 14 ACs)
specs/0014-program-diet-from-archetype.md     ← format and depth to match
mobile/lib/src/workout/                       ← existing SessionDriver, LiveSessionScreen
mobile/pubspec.yaml                           ← packages already in the project
CLAUDE.md                                     ← engineering constitution
project-specifics.md                          ← stack details
```

---

## Step 2 — Decisions already locked (do not re-open)

| Decision | Value |
|----------|-------|
| TTS engine | `flutter_tts` |
| Media-button + background audio | `audio_service` |
| Background audio | Hard requirement — iOS + Android |
| Earbud button action | Single press = log current set + advance |
| Voice mode entry | Toggle on `LiveSessionScreen` |
| Backend changes | None — Flutter-only |

---

## Step 3 — Open questions you must resolve in the spec

These are from `requirements/0027-earbud-guided-training.md` §5:

- **OQ-H1** — Does R-0027 use `audio_service` only for the foreground service
  and media-button hook, or does it implement the full `AudioHandler` interface?
- **OQ-H2** — TTS queue: does each utterance queue (non-interrupting) or
  interrupt the previous one? What happens if the earbud button is pressed during
  a TTS utterance?
- **OQ-H3** — Which method/callback on `SessionDriver` does the voice adapter
  hook into — a listener, a wrapper, or a new dedicated method?
- **OQ-H4** — Does `flutter_tts` require the `audio` background mode in
  `Info.plist`, or does `audio_service` handle it?
- **OQ-H5** — What text and icon does the Android media notification show?

---

## Step 4 — Sections the spec must contain

```
§1  Overview
    - What this builds on top of (R-0009 SessionDriver, R-0014 UserProgram)
    - Architecture in one paragraph

§2  Flutter architecture
    - Class diagram: VoiceSessionAdapter (or equivalent) ↔ SessionDriver ↔ flutter_tts ↔ audio_service
    - State machine: voice mode off → on → (set announced → button pressed → rest cued → next set) → off
    - How toggle interacts with existing LiveSessionScreen state

§3  TTS script (exact strings)
    - Exercise start: "Next: {name}. {N} sets of {R} reps[ at {W} kg]."
    - Set start: "Set {X} of {N}. {R} reps[ at {W} kg]. Go."
    - Rest: "Rest."
    - Workout done: "Workout done."
    - Rules: weight clause omitted when not set; name taken verbatim from session

§4  audio_service integration
    - Minimum AudioHandler implementation required
    - How to register MPRemoteCommandCenter (iOS) and KEYCODE_HEADSETHOOK (Android)
    - How to keep the command center active with screen off

§5  iOS platform config
    - Info.plist background mode key + value
    - AVAudioSession category
    - Entitlement requirement (if any)

§6  Android platform config
    - AndroidManifest.xml foreground service declaration
    - Notification channel: title, icon, importance level
    - Android 14+ permission changes (if any)

§7  Error handling
    - TTS engine unavailable → silent continue, visual indicator
    - Audio focus denied → silent continue
    - Earbud disconnect → release handler, screen-tap still works
    - Reconnect → re-register handler

§8  pubspec.yaml additions
    - flutter_tts: version constraint
    - audio_service: version constraint
    - Any transitive deps to pin

§9  Test plan hooks
    - What the widget tests will mock (TTS service interface, button callback)
    - What the unit tests will verify (VoiceSessionAdapter state transitions)
    - How to simulate earbud button in widget tests

§10 Acceptance criteria mapping
    - One line per AC (AC1–AC14) showing which spec section satisfies it
```

---

## Step 5 — Commit and push

```bash
git checkout R-0027-earbud-training
# write the file
git add specs/0027-earbud-guided-training.md
git commit -m "R-0027: SPEC-0027 draft — earbud-guided training"
git push
```

---

## Done when

`specs/0027-earbud-guided-training.md` exists on `R-0027-earbud-training`,
all 14 ACs are addressed, all 5 OQ-H questions are resolved in the spec text.
