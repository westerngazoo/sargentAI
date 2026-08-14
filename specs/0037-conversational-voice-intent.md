# SPEC-0037 — Conversational Voice Intent

## 1. Overview

This spec realizes R-0037, evolving the existing single-shot voice logging parser (`/voice/intent`) into a multi-turn conversational agent. The client now sends a bounded conversation history to the backend, and the LLM acts as an agent using "tool calls" (via a JSON prompt shim) to either parse an action fully, or return a `clarify` action requesting the missing fields based on the context.

## 2. Architecture

### 2.1 Stateless Server & Client-Carried History
The server remains stateless. The existing `POST /voice/intent` endpoint is updated to accept an optional `history` array in `IntentRequest`. The history is appended to the prompt as prior dialogue to inform the model's decision on the new utterance.
The `ChatTurn` format maps cleanly to the `SergeantState.history` on the mobile client.

### 2.2 Tool-Calling Prompt Shim
To support OpenAI-compatible (e.g. Ollama, Qwen) and Anthropic models without changing the codebase logic per provider, the tool calling is simulated using a strict JSON-mode prompt shim.
The `parse_with_llm` function modifies its prompt to instruct the model to choose between the following "tools" (JSON schemas):
- `log_workout`
- `log_meal`
- `clarify`
- `navigate`

If a command is missing required arguments, the prompt instructs the model to return a `clarify` JSON describing what is missing.

## 3. Implementation Details

### 3.1 Backend Updates
1. `backend/crates/api/src/voice/handlers.rs`:
   - Add a `ChatTurn` struct (`from_user: bool`, `text: String`).
   - Add `history: Option<Vec<ChatTurn>>` to `IntentRequest`.

2. `backend/crates/api/src/voice/parse.rs`:
   - Modify `parse_with_llm` signature to accept the `history`.
   - Update the prompt to format the history context and define the JSON action schemas strictly.

### 3.2 Mobile Updates
1. `mobile/lib/src/hub/voice_intent_service.dart`:
   - Add `history` parameter to `parse(String transcript, {List<ChatTurn>? history})`.
   - Map `ChatTurn` instances to the JSON payload.

## 4. Tests
- Add unit tests in `mobile/test/hub/voice_intent_service_test.dart` to verify that history is correctly added to the JSON payload.
- Update/Add `voice/parse.rs` backend tests to verify behavior when `history` is provided (e.g., following up on a missing protein argument).
