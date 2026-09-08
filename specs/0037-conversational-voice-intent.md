# SPEC-0037: Conversational Voice Intent

## 1. Overview
Realizes R-0037. The voice intent parser shifts from a single-shot keyword/LLM matcher to a multi-turn conversational agent by sending chat history from the client to the stateless backend. The backend uses a model-agnostic JSON-shimming approach to simulate tool-calling for intent resolution.

## 2. Architecture

### 2.1. Client-Side State
The Flutter client (`Sergeant` in `mobile/lib/src/hub/sergeant.dart`) already maintains a list of `ChatTurn`. It will map this to a list of `{ "role": "user"|"assistant", "content": "..." }` and send it in the `/voice/intent` request body alongside the current transcript.

### 2.2. API Contract
`POST /voice/intent`
```json
{
  "transcript": "add 100 grams of chicken",
  "history": [
    { "role": "user", "content": "log a meal" },
    { "role": "assistant", "content": "Tell me the grams of protein, carbs, and fat — a portion like 200 grams of chicken breast — or a preset, like protein shake." }
  ]
}
```

### 2.3. Backend Processing
The backend (`backend/crates/api/src/voice/parse.rs`) formats the history directly into the LLM context prompt as text, instead of passing them as separate message objects, ensuring compatibility across all models (Anthropic, Ollama, Workers AI) without relying on their varying native tool-calling APIs.
The LLM is instructed to return a JSON object (a "tool call") representing the action, matching the existing `ParsedAction` structure.

## 3. Implementation Details

- **Backend `IntentRequest`**: Add `history: Option<Vec<Turn>>`.
- **Prompt formulation**: Append history to the prompt.
- **Tools**: `log_workout`, `log_meal`, `clarify`, `navigate`, `unknown`.
- **Client**: Update `voiceIntentServiceProvider.parse` to accept `List<Map<String, String>> history`.

## 4. Testing
- Unit tests in `voice_intent.rs` to simulate a multi-turn conversation.
