# SPEC-0032 — Voice Assistant and Smart Reminders

- **Status:** Draft
- **Realizes:** R-0032
- **Author: Gustavo Delgadillo**
- **Created: 2026-07-01**
- **Depends on:** none
- **Module(s):** mobile/lib/src/voice, backend/src/api/voice

## 1. Motivation

This spec realizes R-0032. It enables hands-free, natural language logging for workouts and nutrition via an LLM intent parser, and introduces smart reminders for missing logs based on user routine, improving user retention and data quality.

## 2. Design

### 2.1 Voice Input (Flutter)
- A persistent floating action button or `AppBar` action, visible globally across main screens (via `HomeShell`).
- Uses `flutter_sound` or native bindings to record audio (`.wav` or `.m4a`).
- To avoid massive app sizes and complex native C/C++ bindings for on-device STT, audio is sent to the backend.

### 2.2 Backend STT & Intent Parsing
- `POST /voice/log` accepts a multipart audio file.
- The backend delegates STT to a cloud provider (e.g., OpenAI Whisper API).
- The transcript is fed to an LLM (e.g., Claude) with a strict JSON schema prompt to extract intent (`workout` or `nutrition`) and structured fields.
- The backend performs the required database insertions (reusing logic from R-0004 and R-0005) and returns the parsed result.

### 2.3 Smart Reminders
- A cron service (e.g., `tokio::time::interval` in a background task) checks expected logs (e.g., active program workout days, daily meal times).
- Uses Firebase Cloud Messaging (FCM) or native local notifications (`flutter_local_notifications` scheduled locally) to alert the user.

## 3. Code outline

```rust
// backend/src/api/voice.rs
pub async fn log_voice_command(
    State(state): State<AppState>,
    user: Claims,
    mut multipart: Multipart,
) -> Result<Json<VoiceLogResponse>, ApiError> {
    // 1. read audio
    // 2. call whisper api -> transcript
    // 3. call LLM with prompt -> structured JSON
    // 4. insert to DB
    // 5. return success + parsed info
}
```

## 4. Non-goals

- General conversational chatbot capabilities.
- On-device LLM inference.
- Handling arbitrary commands outside of logging nutrition/workouts.

## 5. Open questions

- **OQ-1:** Local notifications vs FCM for reminders? (Likely local for simplicity if routine is static).
- **OQ-2:** Prompt design for reliable macro extraction.

## 6. Acceptance criteria

- [ ] AC1: Global mic button toggles recording state.
- [ ] AC2: Backend endpoint `POST /voice/log` parses transcript and inserts records.
- [ ] AC6: Background job or local scheduler triggers notifications.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| | | |


## Changelog

- _created_
