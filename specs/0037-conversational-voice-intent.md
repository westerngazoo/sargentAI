# SPEC-0037 — Conversational Voice Intent

## 1. Overview

This specification details the transition of the voice logging assistant from a single-shot regex-driven approach to a multi-turn conversational agent powered by tool calling.

As stated in R-0037, real speech is incremental. The current system asks a clarifying question when context is missing but then forgets the original intent, requiring the user to repeat the full command correctly in one breath. This spec introduces client-carried history to let the assistant ask targeted questions and accumulate required arguments over multiple turns before logging the event via JSON tool schemas.

## 2. Core Decisions & Answers to Open Questions

*   **OQ-1 (Turn-window size and token budget):** We cap the history window to the last 6 turns (3 user/assistant pairs) to stay well within token limits and reduce latency.
*   **OQ-2 (Uniform tool-calling format):** To maintain model agnosticism (R-0037 §3.AC6), we use a unified "JSON schema in prompt" approach rather than relying on provider-specific tool-calling APIs. The system prompt instructs the model to return a single JSON object corresponding to an action.
*   **OQ-3 (Default production model):** We keep Anthropic's Claude Haiku (`claude-haiku-4-5`) as the primary production model. OpenAI-compatible endpoints and a keyword parser remain as fallbacks.
*   **OQ-4 (Where bounded history lives):** Turn history lives on the client inside `SergeantState`. It is serialized and sent to the server in the `/voice/intent` request payload. Ending a session ("out") automatically clears this state.
*   **OQ-5 (Confirmation policy):** The assistant will auto-commit (log to the database) when confident it has all required fields, without explicitly asking for a final "yes/no" confirmation, minimizing friction.

## 3. Architecture

### 3.1 Mobile Client
The client (`SergeantState`) maintains an ordered array of `ChatTurn` objects.
During a voice command, the `VoiceIntentService` posts the current `transcript` and the recent `history` array to the server.

### 3.2 Backend Endpoint
`POST /voice/intent` accepts:
```json
{
  "transcript": "chicken breast, 200 grams",
  "history": [
    {"role": "user", "content": "log a meal"},
    {"role": "assistant", "content": "Tell me the grams of protein, carbs, and fat."}
  ]
}
```

The backend maps these roles and contents to the upstream LLM API format. The prompt continues to specify the JSON schema the LLM must return, enforcing domain constraints natively.
