# SPEC-0037 — Conversational Voice Intent (multi-turn, tool-calling)

- **Status:** Implemented
- **Realizes:** R-0037
- **Depends on:** R-0032 (Voice Assistant)
- **Module(s):**
  - `backend/crates/api/src/voice/handlers.rs` — endpoint schema (history)
  - `backend/crates/api/src/voice/parse.rs` — LLM tool calling mapping
  - `mobile/lib/src/hub/sergeant.dart` — pass conversation history to API
  - `mobile/lib/src/hub/voice_intent_service.dart` — payload for `POST /voice/intent`

## 1. Motivation

Realizes [R-0037](../requirements/0037-conversational-voice-intent.md). The
existing voice assistant operates as a single-shot regex fallback or single-turn
LLM parser. Evolving it to a multi-turn conversation that remembers context
allows users to give partial commands (e.g. "log a meal"), have the assistant ask
for missing information (e.g. "How many grams of protein, carbs, and fat?"),
and then provide the remainder (e.g. "100 grams of chicken").

Instead of custom JSON outputs, the LLM uses native tool calling to decide when
it has enough information to execute an intent (log) versus asking a clarifying
question.

## 2. Design

### 2.1 Backend: History and Tool Calling

**Endpoint `POST /voice/intent`**
The `IntentRequest` is extended to accept a `history` array of `{role, content}` objects.

**Prompt and Tools**
The raw JSON prompt is replaced with a system prompt and an array of `tools`:
- `log_workout` (exercise, reps, weight_kg)
- `log_meal` (protein_g, carbs_g, fat_g)
- `clarify` (prompt)
- `navigate` (route, message)

**Provider Seam (`parse_with_llm`)**
The `parse_with_llm` function maps the history and tools to the appropriate schema for the active provider:
- **OpenAiCompatible:** Passes the tools array and messages directly.
- **Anthropic:** Translates OpenAI `tools` schema into Anthropic's `tools` (renaming `parameters` to `input_schema`), and maps `role` correctly (Anthropic restricts system prompt to a dedicated field or the first message context).

### 2.2 Frontend: Context Passing

The `SergeantState` in `mobile/lib/src/hub/sergeant.dart` already maintains a capped `history` of `ChatTurn`s.

**Payload Update:**
`VoiceIntentService.parse` is updated to send this history alongside the current transcript. The `ChatTurn`s are formatted into `{'role': turn.fromUser ? 'user' : 'assistant', 'content': turn.text}`.

## 3. Implementation Details

- **`IntentRequest`:** Added `pub history: Option<Vec<ChatMessage>>`.
- **`ChatMessage`:** `pub role: String, pub content: String`.
- **`extract_llm_tool_call`:** Replaces `extract_llm_text`. Extracts the tool call object based on the provider:
    - OpenAI: `json["choices"][0]["message"]["tool_calls"][0]`
    - Anthropic: Iterates `json["content"]` looking for `type == "tool_use"`.
- **`llm_json_to_action`:** Now receives `tool_name: &str` and `v: &serde_json::Value` (arguments). Match block works exactly the same.