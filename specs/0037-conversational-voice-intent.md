# SPEC-0037 — Conversational Voice Intent

- **Status:** Draft
- **Realizes:** R-0037
- **Owner:** Gustavo Delgadillo

## 1. Overview
The goal is to move from single-turn voice logging to multi-turn voice logging (where the backend requests clarification and the app sends context in each request).

The mobile app must maintain conversational context and pass it to `/voice/intent`. The backend needs an LLM component capable of tool calling (`log_workout`, `log_meal`) and providing conversational follow-ups.

## 2. API Design

We augment the existing `VoiceIntentRequest` payload to support context.

```json
{
  "transcript": "log a meal",
  "history": [
    { "role": "user", "content": "I ate chicken" },
    { "role": "assistant", "content": "How many grams?" }
  ]
}
```

## 3. Tool Calling and LLM Prompt

We will prompt an LLM using few-shot examples or schema to output JSON representing a tool call (either a `log_workout`, `log_meal`, or a `clarify` request).

## 4. Work breakdown

1. Update `VoiceIntentRequest` and `VoiceIntentResponse` models in Rust and Dart.
2. Maintain `history` in `Sergeant` (Dart).
3. Connect the Rust backend to an LLM provider using JSON shimming for tool-calling capabilities. (Anthropic or OpenAI-compatible).
