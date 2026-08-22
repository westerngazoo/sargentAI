# SPEC-0037 — Conversational Voice Intent (multi-turn, tool-calling)

- **Requirement:** [R-0037](../requirements/0037-conversational-voice-intent.md)
- **Status:** Implemented

## 1. Architecture Overview

To support multi-turn conversational voice logging without introducing server-side state, we adopt a stateless architecture where the client maintains the conversation history and sends it along with each new transcript to the backend. The backend LLM parser is updated to accept this history and inject it into the prompt, using JSON-based tool-calling logic to decide whether to complete the log or ask for clarification.

## 2. Client-Carried History

- The Flutter app's `Sergeant` (in `mobile/lib/src/hub/sergeant.dart`) already maintains a limited `history` of `ChatTurn` objects.
- `VoiceIntentService` and the `POST /voice/intent` request payload will be updated to include an optional `history` array.
- Each item in the `history` array will be an object with `role` (either "user" or "assistant") and `content` (the text of the turn).

## 3. Tool Calling via JSON Prompt Shim

- The backend (`backend/crates/api/src/voice/parse.rs`) will format the `history` array into a readable conversation transcript.
- To avoid API validation errors regarding strict alternating role requirements (e.g., Anthropic's restrictions on multiple user messages or leading assistant messages), we inject the entire multi-turn conversation history directly into the single `user` prompt context.
- The prompt will instruct the LLM to output a JSON object representing a tool call (either `log_workout`, `log_meal`, `clarify`, `navigate`, or `unknown`), exactly as it does now, but with the added context of the previous turns.

## 4. Testing

- **Backend:** Update the integration tests in `backend/crates/api/tests/voice_intent.rs` to mock scenarios where missing information in the current turn is provided by the history.
- **Mobile:** The existing logic in the `Sergeant` will seamlessly handle sending the history; widget/unit tests (if applicable) can verify that the service is called with the correct parameters.