# SPEC-0037: Conversational Voice Intent (multi-turn, tool-calling)

## 1. Overview
This spec outlines the realization of R-0037, evolving the voice intent parser to support multi-turn conversational context. It builds on the single-shot parser (R-0032) by maintaining history in the client and sending it to the backend to resolve intents using past dialogue.

## 2. Architecture
- **Stateless Backend:** The backend exposes `POST /voice/intent` which now accepts an optional `history` array of turns (`role` and `content`). The LLM prompt interpolates these turns as raw text context to keep the tool-calling format unchanged.
- **Client-Side State:** The Flutter client's `Sergeant` (via Riverpod) maintains the conversation history as a bounded list of recent chat turns. These turns are mapped to `role` ("user" or "assistant") and `content` before passing to the backend.
- **Tool-calling shim:** Instead of native API tool-calling (to keep it compatible with any OpenAI-compatible provider like Ollama), the backend continues to use a JSON schema in the prompt, with additional instructions to handle follow-ups ("clarify") and use the `history`.

## 3. Implementation Details

### 3.1 Backend
- Update `IntentRequest` in `backend/crates/api/src/voice/handlers.rs` to include `history: Vec<Turn>` with `#[serde(default)]`.
- `Turn` struct has `role: String` and `content: String`.
- `parse_with_llm` in `backend/crates/api/src/voice/parse.rs` accepts `history`. It builds a `dialogue` string from `history` and injects it into the prompt.
- Tests should be updated to ensure `parse_with_llm` handles the new prompt correctly.

### 3.2 Mobile
- Update `VoiceIntentService.parse` to accept `List<Map<String, String>> history`.
- Update `Sergeant` to map its internal `history` to the format required and pass it to `VoiceIntentService.parse`.
