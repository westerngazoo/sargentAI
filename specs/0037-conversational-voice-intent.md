# SPEC-0037 — Conversational Voice Intent

## 1. Overview

This specification details the technical realization of R-0037. It outlines how the single-shot voice intent parser is extended to support multi-turn conversational context, allowing the LLM to request missing information and accumulate context over multiple turns until enough data is gathered to execute a logging action.

## 2. Architecture

The architecture maintains the stateless nature of the backend `/voice/intent` endpoint. The mobile client (via `Sergeant`) is responsible for keeping track of the conversation history.

### 2.1 Mobile Client (Flutter)

1.  **State Management**: `SergeantState` maintains a `history` list of `ChatTurn` tuples `({bool fromUser, String text})`, capped at 12 turns.
2.  **API Payload**: When the user speaks a new command, the `VoiceIntentService` packages the current transcript along with the existing `history` and sends it in the JSON body to `POST /voice/intent`. The payload shape is:
    ```json
    {
      "transcript": "...",
      "history": [
        {"from_user": true, "text": "log a meal"},
        {"from_user": false, "text": "How many grams of protein, carbs, and fat?"},
        {"from_user": true, "text": "40 protein, 60 carbs, 20 fat"}
      ]
    }
    ```
3.  **Local Fallback**: The local keyword parser remains untouched and functions as a fallback when the backend fails or is unreachable. It only operates on the latest transcript (single-shot).

### 2.2 Backend (Rust)

1.  **Request Model**: The `IntentRequest` struct in `voice/handlers.rs` is updated to accept the optional `history` field using `#[serde(default)]`.
    ```rust
    #[derive(Debug, Deserialize)]
    pub(crate) struct ChatTurn {
        pub from_user: bool,
        pub text: String,
    }
    ```
2.  **LLM Prompting**: The multi-turn conversation history is formatted into text and injected directly into the LLM context prompt in `voice/parse.rs` (JSON prompt shimming). This avoids complexity with provider-specific message array formats and ensures the single prompt approach continues to work across Anthropic, Ollama, etc.
    The prompt structure will be:
    ```
    You parse gym voice commands into JSON only. Today is {today}.

    Previous conversation:
    User: log a meal
    Assistant: Tell me the macros.

    Current Transcript: "40 protein, 60 carbs, 20 fat"

    Return ONE of: ...
    ```
3.  **Function/Tool Calling**: The instructions explicitly tell the LLM to emit the `clarify` action if it needs missing information, or emit the completed action (`log_workout` / `log_meal`) if the combination of previous context and current transcript provides all required data.

## 3. Open Questions Resolved

*   **OQ-1 (Turn-window size):** Capped at 12 turns on the client.
*   **OQ-2 (Uniform tool-calling format):** JSON-schema-in-prompt shim (existing approach) is preserved, simply augmented with the text-formatted history block.
*   **OQ-4 (History lifecycle):** Managed in `SergeantState` and cleared when the user navigates away or says a stop command.
