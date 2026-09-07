# SPEC-0037: Conversational Voice Intent

## 1. Overview
This specification details the implementation for multi-turn voice context and tool-calling capabilities as outlined in R-0037. It replaces the single-shot prompt with a system that can follow up with the user when information is missing.

## 2. Architecture

### 2.1 Backend Changes
The `POST /voice/intent` endpoint will be modified to accept a `history` array containing previous conversation turns.
The backend remains stateless. The mobile app will send the conversation context on each request.

**Payload Structure:**
```json
{
  "transcript": "chicken breast, 200 grams",
  "history": [
    {"role": "user", "content": "log a meal"},
    {"role": "assistant", "content": "What did you eat and how much?"}
  ]
}
```

The system will format this `history` directly into the LLM context prompt instead of passing them as separate system/user message objects for broader compatibility across OpenAI and Anthropic compatible providers without needing complex role mappings.

### 2.2 Model Provider Shim & Tool Calling
To maintain model agnosticism across Anthropic, Ollama, and OpenAI endpoints, tool calling will be implemented via a JSON-schema-in-prompt shim (OQ-2). The prompt will instruct the LLM to output specific JSON actions (`log_workout`, `log_meal`, `clarify`, `navigate`). The LLM is instructed to use `clarify` if required properties (like protein, carbs, fat, or weight and reps) are missing, effectively becoming our "tool calling" mechanism.

### 2.3 Mobile App Changes
- `SergeantState` inside `mobile/lib/src/hub/sergeant.dart` already tracks history in `history` (`List<ChatTurn>`).
- We will modify `VoiceIntentService` in `mobile/lib/src/hub/voice_intent_service.dart` to accept this history and send it in the JSON payload.
- The history window will be capped to 12 turns (OQ-1) which `Sergeant` currently enforces.
- `Sergeant` naturally clears history on session-end when the state provider is torn down/rebuilt.

## 3. Implementation Plan
1. **Dart Data Classes**: Create a Dart class for history elements (`ConversationTurn`) and update `VoiceIntentService` to pass it. Map `ChatTurn` to `ConversationTurn`.
2. **Rust API Types**: Update `IntentRequest` in `backend/crates/api/src/voice/handlers.rs` to include `history`.
3. **Prompt Update**: Modify `parse_with_llm` in `backend/crates/api/src/voice/parse.rs` to inject the conversation history directly into the system prompt and update the JSON action instructions to encourage `clarify` when parameters are missing.
4. **Testing**: Add backend tests verifying the `clarify` response for incomplete inputs, and successful parsing when history completes the context.
