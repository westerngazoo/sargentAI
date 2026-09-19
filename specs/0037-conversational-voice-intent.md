# SPEC-0037 — Conversational Voice Intent

- **Status:** Draft
- **Realizes:** R-0037
- **Author:** Jules
- **Created:** 2026-07-09
- **Depends on:** R-0032
- **Module(s):** backend (Axum API) + mobile (Flutter)

## 1. Motivation

This spec realizes R-0037, evolving the single-shot STT voice parser into a multi-turn LLM loop. Currently, incomplete dictations (e.g., "log a meal") result in a single clarification response, requiring the user to re-state the entire command or give up. By carrying conversation history within the JSON request, the LLM can use prior context to infer missing details, turning logging into a short dialogue.

## 2. Design

- **Stateless Server Architecture**: The backend will not maintain conversation state (no sessions/websockets). Instead, the Flutter client will maintain a bounded conversation history and submit it with every `/voice/intent` request.
- **Model-Agnostic LLM Prompting**: The system currently supports multiple providers (Anthropic, Ollama, etc.) through a custom JSON structure. Instead of using complex native tool-calling features (which differ across providers), the backend will inject the chat history directly into the LLM context prompt as text, before the latest transcript.
- **Flutter UI Integration**: The `SergeantState` already retains `history` as a list of `ChatTurn` (`{bool fromUser, String text}`). The `VoiceIntentService` will serialize this into `[{"role": "user"|"assistant", "content": "..."}]` and include it in the POST request. Upon a successful log (`logged_workout`, `logged_nutrition`) or navigation, the history will be cleared to prevent context pollution in subsequent commands.

## 3. Code outline

**Backend (`backend/crates/api/src/voice/parse.rs`):**

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// In `handlers.rs` IntentRequest:
#[derive(Debug, Deserialize)]
pub(crate) struct IntentRequest {
    pub transcript: String,
    #[serde(default)]
    pub history: Vec<crate::voice::parse::Message>,
}
```

**Flutter (`mobile/lib/src/hub/sergeant.dart`):**

```dart
// Map existing history
final historyJson = state.history.map((turn) => {
  'role': turn.fromUser ? 'user' : 'assistant',
  'content': turn.text,
}).toList();

// Service call
final result = await ref.read(voiceIntentServiceProvider).parse(
  transcript,
  history: historyJson,
);
```

## 4. Non-goals

- A server-side session store (e.g. Redis, Durable Objects).
- Supporting general chatbot queries outside of fitness/nutrition.
- Upgrading STT to stream audio instead of using on-device text.

## 5. Open questions

- None at the moment; design defers to the existing model-agnostic JSON approach as requested.

## 6. Acceptance criteria

- [x] Backend parses missing field clarify requests with history.
- [x] Backend `IntentRequest` supports optional history without breaking older clients.
- [x] Flutter client passes bounded history array over the network.
- [x] Successful multi-turn logging parses properly via the LLM prompt construction.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Inject history as text into system prompt | Simplifies compatibility across different providers (Anthropic vs OpenAI-compatible endpoints) without needing bespoke tool-call shimming. |

## Changelog

- _created_
