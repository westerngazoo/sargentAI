# SPEC-0037: Conversational Voice Intent (multi-turn, tool-calling)

## Overview

This specification realizes R-0037, evolving the existing single-shot `/voice/intent` endpoint into a multi-turn, context-aware dialogue system powered by native LLM tool-calling.

## Architecture

The system remains stateless on the server. The mobile client holds the short-term conversation history and passes it on every request. The LLM dictates control flow by emitting a tool call when confident (e.g. `log_meal`) or returning a clarification text/tool when lacking information.

## Backend Changes (`/voice/intent`)

### Request Payload

The POST request accepts an optional `history` array of previous turns:

```json
{
  "transcript": "chicken breast",
  "history": [
    { "role": "user", "content": "log a meal" },
    { "role": "assistant", "content": "How many grams of protein, carbs, and fat?" }
  ]
}
```

### LLM Integration (Tool Calling)

Instead of the raw JSON-mode prompt (which was brittle and single-shot), we define native tools for Anthropic (Claude) and OpenAI-compatible models (Ollama, Workers AI, vLLM).

**Tools defined:**
1. `log_workout(exercise, reps, weight_kg?)`
2. `log_meal(protein_g, carbs_g, fat_g)`
3. `clarify(prompt)`
4. `navigate(route, message)`

The prompt instructs the model to use the chat history to resolve the user's latest `transcript` and to use the `clarify` tool if required fields (like macros for a meal) are missing, instead of hallucinating values or guessing.

## Mobile Client Changes

1.  **`VoiceIntentService`**: Updated to accept a `history` list (from `SergeantState`) and map the mobile `ChatTurn` (`fromUser` boolean, `text` string) to the API's expected `role` and `content`.
2.  **`Sergeant`**: The existing `state.history` logic is leveraged. On unknown intents or when the backend returns a `clarify` action, the `Sergeant` keeps the conversation alive and appends the turns.

## Fallback and Degradation

If the LLM parsing fails or is disabled (no API key/URL), the system falls back to the existing regex/keyword-based parser. The keyword parser will ignore the history payload and operate in single-shot mode, maintaining the current baseline experience.

## Testing Strategy

-   **Backend Integration Tests (`tests/voice_intent.rs`)**: Send multi-turn requests (simulating a missing argument leading to a `clarify` response, followed by providing the argument and getting a `logged_meal` response).
-   **Unit Tests (`parse.rs`)**: Test the tool-call extraction logic for both `Anthropic` and `OpenAiCompatible` providers.
-   **Mobile Tests**: Ensure `Sergeant` correctly maps and passes its state history down to the service layer.
