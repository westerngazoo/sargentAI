# SPEC-0037 — Conversational Voice Intent (multi-turn, tool-calling)

## Overview

This spec realizes `R-0037` by extending the existing `/voice/intent` endpoint and the Flutter `Sergeant` service to handle conversational context. It builds on the single-shot LLM integration from `R-0032`, passing history back and forth to give the agent a conversational memory, allowing it to act on incomplete utterances by requesting clarifications.

## Answers to Open Questions

- **OQ-1 (Turn-window size):** Capped at 12 turns (as currently tracked in `SergeantState.history`). No need for an explicit truncation on the backend; the client manages its capped window.
- **OQ-2 (Wire format):** Due to varied model support and the established architecture (see memory directives), we will use a **JSON-schema-in-prompt shim**. The conversation history is directly formatted as text (e.g., `User: ... \n Assistant: ...`) and appended to the context prompt, rather than passed as native message arrays.
- **OQ-3 (Default production model):** We retain the existing `LlmConfig` mechanism. The default configured LLM from the environment (or Ollama/fallback) is used. No special fallback tier logic is needed beyond the existing keyword parser.
- **OQ-4 (Bounded history location):** The history lives in the Flutter client inside `SergeantState` (which already defines `history: List<ChatTurn>`). It resets naturally with `isOut(transcript)` or session navigation.
- **OQ-5 (Confirmation policy):** Auto-commit when confident. When the LLM outputs a `log_workout` or `log_meal` action with all required arguments, the backend immediately commits the record without a separate confirmation turn.

## Architecture

### Backend (`backend/crates/api/src/voice/`)

- **Statelessness:** The backend remains stateless.
- **`IntentRequest`:** Extended to accept an optional `turns` array of `{ role: String, content: String }`. The `#[serde(default)]` attribute ensures backward compatibility.
- **Prompt construction:** `parse_with_llm` incorporates `turns` directly into the string prompt to feed into the model context before evaluating the new `transcript`.

### Mobile (`mobile/lib/src/hub/`)

- **`VoiceIntentService`:** Extended to serialize `VoiceTurn` instances alongside the `transcript`.
- **`Sergeant`:** Maps its `history` (of type `ChatTurn`) into `VoiceTurn` JSON payloads, passing the conversation state so the model knows what macros or details were previously discussed.

## Acceptance Criteria Traceability

- **AC1:** History travels in JSON as `turns`.
- **AC2:** The system prompt dictates JSON responses simulating tool calls.
- **AC3:** The `clarify` action (already present) handles missing fields.
- **AC4:** Valid `log_*` outputs commit instantly.
- **AC5:** Strict `New*` constructor validation remains untouched.
- **AC6:** Relies on the existing `LlmConfig` abstraction.
- **AC7:** Keyword parser handles fallback if the LLM request fails.
- **AC8:** The existing `unknown` JSON handler maintains the fitness scope.
- **AC9:** Tested via explicit `turns` unit and integration tests.
