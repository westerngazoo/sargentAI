# SPEC-0037 — Conversational Voice Intent

- **Status:** Accepted
- **Realizes:** R-0037
- **Author:** Jules
- **Created:** 2026-07-09
- **Depends on:** R-0032
- **Module(s):**
  - `mobile/lib/src/hub/voice_intent_service.dart`
  - `mobile/lib/src/hub/sergeant.dart`
  - `backend/crates/api/src/voice/handlers.rs`
  - `backend/crates/api/src/voice/parse.rs`

## 1. Motivation

Realizes R-0037: evolve the voice logger from a single-shot regex/JSON parser into a multi-turn conversation agent using LLM tool calling. This allows the assistant to ask for missing required fields contextually and gracefully commit once all fields are collected across multiple turns.

## 2. Design

- **Client state:** `SergeantState.history` (a capped list of `ChatTurn`) already stores the conversation. We map this history to `[{"role": "user"/"assistant", "content": "..."}]` and send it alongside the `transcript` to `POST /voice/intent`.
- **Backend request:** `IntentRequest` adds `history: Option<Vec<Turn>>`.
- **Tool Calling vs JSON mode (OQ-2):** We will use the native tool-calling capabilities of the respective API providers (Anthropic `tools` + OpenAI `tools`). This leverages their tuned function-calling capabilities.
- **Provider Translation:**
  - **OpenAI-Compatible:** Sent with `"tools": [...]`. Responses return `tool_calls`.
  - **Anthropic:** Sent with `"tools": [...]`. Responses return `tool_use` blocks.
- **Tools Defined:**
  - `log_workout(exercise, reps, weight_kg)`
  - `log_meal(protein_g, carbs_g, fat_g)`
  - `clarify(prompt)`
  - `navigate(route, message)`
- **Graceful Degradation:** If the LLM call fails, we still fall back to `parse_transcript` (keyword parser) using the current `transcript`.

## 3. Code outline

```rust
#[derive(Debug, Deserialize)]
pub(crate) struct Turn {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IntentRequest {
    transcript: String,
    history: Option<Vec<Turn>>,
}
```

## 4. Non-goals

- Server-side persistence of conversation history (session store). We rely on client-carried history for this v1.
- No changes to the `speech_to_text` pipeline.

## 5. Open questions

- **OQ-1: Turn-window size:** Capped at 12 by the client (`_appended` limits it).
- **OQ-2: Uniform tool-calling wire format:** Native tool calling for both APIs, abstracted in `extract_llm_tool_call()`.
- **OQ-3: Default production model:** `claude-haiku-4-5` remains default.
- **OQ-4: Where history lives:** Stays in `SergeantState`.
- **OQ-5: Confirmation policy:** Auto-commit on confident tool call.

## 6. Acceptance criteria

- [ ] AC1: Client sends `history` to the backend.
- [ ] AC2: Backend uses LLM tool calling (not regex/JSON-mode).
- [ ] AC3: Missing fields result in a `clarify` tool call.
- [ ] AC4: Commits when complete (`log_workout` / `log_meal`).
- [ ] AC5: Falls back to keyword parser on LLM error.
- [ ] AC9: Backend unit tests cover tool call responses.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Use native tool calling | Tuned function-calling models perform better than JSON schema prompts for multi-turn extraction. |
