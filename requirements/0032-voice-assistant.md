# R-0032 — Voice Logging Assistant

- **Status:** Accepted (amended to as-built 2026-07-06, R-0057)
- **Milestone:** M9 (Voice Assistant & Automation)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-22
- **Depends on:** R-0004 (Workout log), R-0005 (Nutrition log), R-0009 (Live workout logger)
- **Realized by:** [SPEC-0032](../specs/0032-voice-assistant.md)
- **QA:** `qa` agent run scoped to this requirement

---

> **Amendment note (R-0057, 2026-07-06).** This feature merged via PRs #39/#49/#50
> ahead of its spec (the spec drafted in PR #37 described a different, unbuilt
> architecture and is superseded). During reconciliation the owner **scoped this
> requirement down to the voice-logging half that actually shipped** and **spun
> the smart-reminders half out to [R-0036](0036-voice-reminders.md)**. The
> original reminder criteria (former AC6–AC8) now live in R-0036. Title changed
> from "Voice Assistant and Smart Reminders" to "Voice Logging Assistant".

## 1. Statement

A voice logging assistant reached via a microphone button. Instead of typing to
log meals or workouts, the user speaks natural language (e.g. "Log 150 grams of
chicken breast for lunch" or "I just did 10 reps of 100 kg bench press"). The
app transcribes the speech on-device, parses the intent, and logs it
automatically — prompting for clarification when required fields are missing.

## 2. Rationale

Manual data entry is a point of friction, leading to missed logs and incomplete
data for the ML model. Hands-free, conversational logging improves compliance
and data quality. (Proactive missing-log reminders — a separate concern — are
tracked in R-0036.)

## 3. Acceptance criteria (as-built)

- **AC1. Voice input button.** A microphone entry point launches voice
  interaction (from Home and inside the voice hub).
- **AC2. Speech-to-text.** The app captures speech and transcribes it to text
  **on-device** (via the `speech_to_text` plugin behind a `SpeechInput` seam).
- **AC3. Intent parsing.** The transcript is sent to `POST /voice/intent`, which
  uses an LLM (Anthropic `claude-haiku-4-5`, key-gated) to parse intent and
  extract structured data, with an always-present keyword-parser fallback.
- **AC4. Automatic logging.** Parsed intents create the corresponding nutrition
  or workout log entries automatically.
- **AC5. Confirmation / fallback.** If the intent is ambiguous or missing
  required fields (e.g. "Log some chicken"), the assistant prompts for
  clarification before logging.
- **AC6. Tests.** Flutter widget tests cover the mic button and voice-listening
  UI states (via a `FakeSpeechInput`); backend tests cover intent parsing and
  DB-record creation from structured outputs, including the clarify path.
- **AC7. Privacy / scope guard.** Voice audio is not stored; only the transcript
  and extracted structured data are transmitted/retained. The assistant is
  constrained to fitness/nutrition logging — not a general chatbot. The mic is
  only active when the user initiates it (no always-on background listening).

## 4. Constraints & non-goals

- On-device STT — no raw-audio upload endpoint, no server-side transcription.
- Not a general conversational AI.
- **Out of scope (moved to R-0036):** scheduled missing-log evaluation, reminder
  notifications, and voice-activation from a notification.

## 5. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-22 | (original) single requirement covering voice logging **and** smart reminders | Initial framing. |
| 2026-07-06 | **Split (R-0057):** keep voice logging here; move reminders to R-0036 | The logging half shipped + is tested; the reminder half was never built. Splitting lets logging be signed off honestly. |
| 2026-07-06 | On-device STT + `POST /voice/intent` (JSON), Claude `claude-haiku-4-5` + keyword fallback | Reflects what shipped; supersedes PR #37's backend-Whisper `POST /voice/log` design. |

## Changelog

- _2026-06-22 — created (Status Discussing)._
- _2026-07-06 — **amended to as-built and split** under R-0057; reminders → R-0036; SPEC-0032 written (retro-spec); status → Accepted._
