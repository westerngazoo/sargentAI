# SPEC-0037 — Conversational Voice Intent (multi-turn, tool-calling)

- **Status:** Implemented
- **Realizes:** R-0037
- **Author:** Jules
- **Created:** 2026-07-09
- **Depends on:** SPEC-0032
- **Module(s):** `backend/crates/api/src/voice`, `mobile/lib/src/hub`

## 1. Motivation

Evolve voice logging from a single-shot parser into a multi-turn conversation that holds context and asks for what's missing.

## 2. Design

- **Client-carried history**: `SergeantState.history` on the Flutter client is mapped to an array of `{role, content}` objects and sent to `POST /voice/intent` alongside the current transcript.
- **Backend LLM parsing**: The `IntentRequest` accepts `history`. `parse_with_llm` formats the history directly into the LLM context prompt (as a flat string of turns) before the user's transcript. The LLM behaves as an agent equipped with tool-calling capabilities represented as structured JSON outputs.

## 3. Code outline

```rust
// backend/crates/api/src/voice/handlers.rs
#[derive(Debug, Deserialize)]
pub(crate) struct Turn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IntentRequest {
    pub transcript: String,
    #[serde(default)]
    pub history: Option<Vec<Turn>>,
}
```

```dart
// mobile/lib/src/hub/voice_intent_service.dart
class VoiceIntentService {
  Future<VoiceIntentResult> parse(String transcript, List<Map<String, String>> history) async {
    // ...
  }
}
```

## 4. Non-goals

- A server-side session store / edge agent is an explicit non-goal for v1.
- Not a general assistant; strictly logging + navigation intents.
- No new audio pipeline; on-device STT is unchanged.

## 5. Open questions

All settled.

## 6. Acceptance criteria

- [x] AC1: Multi-turn context.
- [x] AC2: Tool calling.
- [x] AC3: Asks for missing fields.
- [x] AC4: Commits when complete.
- [x] AC5: Validation still enforced.
- [x] AC6: Model-agnostic.
- [x] AC7: Graceful degradation.
- [x] AC8: Scope guard / safety.
- [x] AC9: Tests.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Client-carried history | Defer backend state complexity until warranted |

## Changelog

- _created_
