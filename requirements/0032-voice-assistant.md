# R-0032 — Voice Assistant and Smart Reminders

- **Status:** Discussing
- **Milestone:** M9 (Voice Assistant & Automation)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-22
- **Depends on:** R-0004 (Workout log), R-0005 (Nutrition log), R-0009 (Live workout logger)
- **Realized by:** SPEC-0032 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The app features a persistent voice assistant accessible via a microphone button. Instead of typing to log meals or workouts, the user can speak natural language (e.g., "Log 150 grams of chicken breast for lunch" or "I just did 10 reps of 100kg bench press"). The app processes the speech, maps it to structured inputs, and logs it automatically. Additionally, a smart alert system proactively reminds the user to log missing meals or workouts based on their daily routine, ensuring consistent progress tracking.

## 2. Rationale

Manual data entry is a point of friction for many users, leading to missed logs and incomplete data for the ML model. By allowing hands-free, conversational logging and proactively reminding users of missing entries, we improve user compliance, data quality, and overall retention. This evolves the app from a passive tracker to an active, intelligent assistant.

## 3. Acceptance criteria

- **AC1. Voice Input Button:** A persistent microphone button is accessible across primary screens in the Flutter app to initiate voice interaction.
- **AC2. Live Session Hands-Free Commands:** During an active workout session, the user can speak commands hands-free (e.g., "Finished squats set 1, reps 10, weight 40kg"). The app registers the log and precise timing.
- **AC3. Audio Ducking & Background Playback:** The voice assistant operates seamlessly alongside other audio sources (like background music). When listening or speaking, it uses audio ducking (lowering music volume) without completely stopping the user's media, potentially via native OS integration (iOS `AVAudioSession` mix/duck options, Android AudioFocus).
- **AC4. Energy Expenditure Calculation:** When a set is logged via voice with specific weight, reps, and timing, the backend calculates biomechanical work done (joules) and converts this to estimated active calories burned, accounting for time under tension and rest periods between sets.
- **AC5. Speech-to-Text Processing:** The app captures the user's speech and accurately transcribes it into text using an on-device or cloud STT engine.
- **AC6. Intent Parsing (LLM):** The transcribed text is sent to a backend endpoint that uses an LLM to parse the intent (e.g., logging a meal, logging a workout) and extract structured data (food name, grams, exercise name, reps, weight).
- **AC7. Automatic Logging:** Once parsed, the backend automatically creates the corresponding nutrition or workout log entries.
- **AC8. User Confirmation/Fallback:** If the intent is ambiguous or missing required fields (e.g., "Log some chicken"), the assistant prompts the user for clarification before logging.
- **AC9. Missing Log Reminders:** A scheduled background job or chron service evaluates the user's daily routine (e.g., expected meal times, scheduled workout days from their active `UserProgram`).
- **AC10. Alert System:** If a scheduled meal or workout is not logged within a configurable grace period, the app sends a local or push notification reminding the user.
- **AC11. Voice-Activated from Notification:** Users can tap the reminder notification to immediately open the app into voice-listening mode.
- **AC12. Tests:** Flutter widget tests verify the presence of the microphone button, audio ducking interactions, and the voice-listening UI states. Backend tests verify the prompt construction for the LLM intent parser, the energy expenditure formula, and the creation of database records from structured LLM outputs.
- **AC13. Privacy and Scope Guard:** Voice audio is not stored long-term. Only the extracted structured data is retained.

## 4. Constraints & Non-goals

- **No Always-On Listening:** The microphone is only active when the user explicitly taps the button.
- **Not a General Chatbot:** The assistant is strictly constrained to fitness and nutrition logging capabilities. General conversational AI is out of scope.

## 5. Open questions

Deferred to SPEC-0032:
- **OQ-1:** Which STT (Speech-to-Text) engine to use? Options include native OS APIs via Flutter plugins (e.g., `speech_to_text`) or a cloud service (e.g., Whisper API).
- **OQ-2:** Prompt design and structure for the LLM intent parser (likely Claude) to reliably extract macro and workout data.
- **OQ-3:** How to model the user's "daily routine" for the reminder system (e.g., inferred from past logs vs. explicitly configured meal times).
- **OQ-4:** Implementation details for the chron service triggering alerts.
