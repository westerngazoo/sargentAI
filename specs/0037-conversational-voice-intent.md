# SPEC-0037 — Conversational Voice Intent

- **Status:** Draft
- **Realizes:** R-0037
- **Author:** Jules
- **Created:** 2026-07-30
- **Depends on:** R-0032
- **Module(s):** backend (voice), mobile (hub)

## 1. Motivation

We are replacing the single-turn voice intent parser with a multi-turn conversational approach to allow the LLM to follow up when missing information is needed to log an entry. This will improve completion rate and feel like talking to a real coach.

## 2. Design

**Client-side (Mobile)**:
- Mobile app passes the history to the backend endpoint `/voice/intent`.
- We'll update the `VoiceIntentService` in Flutter to serialize `history: [{"from_user": true, "text": "..."}]`.
- `Sergeant` state already maintains `history` as `List<ChatTurn>`. We pass this history to the API.

**Server-side (Backend)**:
- `IntentRequest` takes `history`.
- `parse_with_llm` incorporates the history into the messages sent to the LLM.
- We use native tool calling for models that support it, but since we want it model-agnostic and OpenAI-compatible (Ollama) has varying support, we will continue using a JSON-schema-in-prompt shim (JSON mode) to simulate tool calling. The prompt explicitly describes the tools: `log_workout`, `log_meal`, and the `clarify` (ask a question) tool, enforcing that the model must use the clarify tool if required fields are missing.
- The model can return: `log_workout`, `log_meal`, `clarify`, `navigate`, or `unknown`.

## 3. Code outline

```rust
// backend/crates/api/src/voice/handlers.rs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ChatTurn {
    pub from_user: bool,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IntentRequest {
    pub transcript: String,
    #[serde(default)]
    pub history: Vec<ChatTurn>,
}
```

```dart
// mobile/lib/src/hub/voice_intent_service.dart
class VoiceIntentService {
  Future<VoiceIntentResult> parse(String transcript, [List<ChatTurn> history = const []]) async {
    // send transcript and mapped history
  }
}
```

## 4. Non-goals

- A server-side session store (e.g. Durable Objects). State is managed on the client.
- Smart reminders (R-0036).
- New audio pipeline.

## 5. Open questions

- **OQ-1:** Turn-window size and token budget: We will limit history to the last 12 items (6 turns) which is already done by `SergeantState`.
- **OQ-2:** Uniform tool-calling wire format across providers: We will use a JSON-schema-in-prompt shim since OpenAI-compatible endpoints like Ollama have varying native tool calling support.
- **OQ-3:** Default production model: `claude-haiku-4-5` with `llama3.2` as a fallback.
- **OQ-4:** Where the bounded history lives: in the `SergeantState` on the mobile client. It gets cleared when `isOut` is triggered.
- **OQ-5:** Confirmation policy: Auto-commit when confident.

## 6. Acceptance criteria

- [ ] AC1. Multi-turn context.
- [ ] AC2. Tool calling, not regex.
- [ ] AC3. Asks for missing fields.
- [ ] AC4. Commits when complete.
- [ ] AC5. Validation still enforced.
- [ ] AC6. Model-agnostic.
- [ ] AC7. Graceful degradation.
- [ ] AC8. Scope guard / safety.
- [ ] AC9. Tests.
- [ ] AC10. Reminders out of scope.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-30 | JSON-schema-in-prompt | Better consistency across various Ollama models without native tool support. |

## Changelog

- _created_
