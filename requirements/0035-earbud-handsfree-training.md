# R-0035 — Earbud-Guided Hands-Free Training (Rebuild)

- **Status:** Accepted
- **Milestone:** M3 (fast-track) / mobile — differentiator
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-06
- **Depends on:** R-0009 (Live workout logger — the session this drives),
                  R-0014 (Program/diet — source of the per-exercise plan to narrate)
- **Realized by:** SPEC-0035 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

> **Origin (R-0057, 2026-07-06).** R-0027 specified an earbud media-button +
> background-audio transport with an explicit *"no speech recognition"* scope
> guard. It was built (PR #48) then **reverted** by conflict-resolution commit
> `6b5bb7e` ("prefer local"), which replaced it with the R-0032 voice-**dictation**
> coach — directly violating R-0027's scope guard and removing the hands-free
> earbud differentiator from `main`. This requirement **rebuilds that transport**
> as a distinct feature that **coexists with** the R-0032 dictation coach (they
> are different input modes on the same live session). The reverted #48 code is
> the starting point.

## 1. Statement

During a live workout, the user can train hands-free with earbuds and the phone
in a pocket / screen off. The app narrates the session over audio (TTS) and
advances through sets/exercises on a **single earbud media-button press** — no
looking at the screen, no speaking. This is the hands-free differentiator: a
coach in your ear that you drive with the earbud button.

## 2. Rationale

Target users train in gyms with the phone stashed. Speaking commands aloud
(R-0032 dictation) is awkward in a crowded gym; a media-button press is
discreet, reliable, and works with any Bluetooth earbuds. This is a stated
product differentiator (earbud-guided training) that must not stay regressed.

## 3. Acceptance criteria

- **AC1.** A session-local "hands-free / earbud" mode toggle on the live session
  screen enables/disables the transport; state is transient (not persisted).
- **AC2.** On entering an exercise, the coach announces it over TTS, including
  the planned targets when available ("Next: bench press. 3 sets of 8 at 80
  kilos.").
- **AC3.** Before each set, a pre-set cue is spoken ("Set 2 of 3. 8 reps at 80
  kilos. Go.").
- **AC4.** A single media-button press (iOS `MPRemoteCommandCenter` play/pause or
  toggle; Android `KEYCODE_HEADSETHOOK` / `MediaSession`) advances the session
  exactly one step (log current set → next set/exercise).
- **AC5.** "Rest." is announced after each logged set.
- **AC6.** The next exercise is announced on advance; "Workout complete." at the
  end.
- **AC7.** Background audio: narration and button handling keep working with the
  screen off and the app backgrounded (iOS `UIBackgroundModes: audio`; Android
  foreground service via `audio_service` or equivalent).
- **AC8.** Both the earbud button and the on-screen "Log set" control advance the
  session identically.
- **AC9.** Toggling the mode off mid-session stops TTS, releases the media
  handler, and preserves session state.
- **AC10.** Program-aware narration when a plan exists; a sensible free-form
  fallback when the program carries no per-set targets. (Note: today's program
  model exposes only exercise **names** — surfacing real per-set targets may
  require a small program-model extension, tracked in SPEC-0035.)
- **AC11.** TTS failure degrades silently (no crash; session continues).
- **AC12.** Earbud disconnect releases the handler; reconnect re-registers it.
- **AC13.** Tests: unit/widget coverage including that the media-button callback
  advances the session identically to the on-screen control, and that toggling
  off releases the handler.
- **AC14.** Scope guard — Flutter-only (plus any program-model read); **no speech
  recognition and no voice commands in this mode** (that is R-0032's separate
  mode). This restores R-0027's original guard.

## 4. Constraints & non-goals

- Coexists with the R-0032 voice-dictation coach; does not remove it. The two
  are selectable input modes.
- No backend/DB change required for the transport itself (a program-model
  extension for per-set targets, if needed, is a separate small change).
- Does not re-introduce always-on microphone listening.

## 5. Open questions (deferred to SPEC-0035)

- **OQ-1:** Restore the reverted `audio_service` / `VoiceSessionAdapter` stack
  from #48 as-is, or re-implement on a current audio-session library?
- **OQ-2:** Does the program model gain real per-set targets (sets/reps/weight)
  to satisfy AC2/AC3, or does the coach narrate names + last-session weights as a
  first cut?
- **OQ-3:** How do the earbud (R-0035) and dictation (R-0032) modes share the
  live-session driver without conflicting over the audio session?

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-06 | Rebuild the earbud transport as R-0035, coexisting with R-0032 dictation | The hands-free earbud differentiator was reverted from `main`; rebuild it without removing the dictation mode. |
| 2026-07-06 | Restore R-0027's "no speech recognition" guard for this mode | Media-button advance is the point; dictation is the other mode. |

## Changelog

- _2026-07-06 — created and **Accepted** (R-0057); supersedes the reverted earbud portion of R-0027._
