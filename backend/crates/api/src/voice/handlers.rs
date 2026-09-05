//! `POST /voice/intent` — parse transcript and auto-log when possible.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    voice::parse::{self, IntentResponse, ParsedAction},
    AppState,
};

#[derive(Clone, Default)]
pub struct VoiceIntentSettings {
    /// Configured LLM endpoint, or `None` to use the keyword parser. Built from
    /// env — `LLM_PROVIDER=ollama` points it at a local OpenAI-compatible server.
    llm: Option<Arc<parse::LlmConfig>>,
    pub http: reqwest::Client,
}

impl VoiceIntentSettings {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            llm: parse::LlmConfig::from_env().map(Arc::new),
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct IntentRequest {
    transcript: String,
}

pub(crate) async fn intent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<IntentRequest>, JsonRejection>,
) -> ApiResult<Json<IntentResponse>> {
    let Json(req) = req.map_err(|_| ApiError::Validation { field: "body" })?;
    let today = Utc::now().date_naive();
    let action = if let Some(cfg) = state.voice.llm.as_deref() {
        parse::parse_with_llm(&state.voice.http, cfg, &req.transcript, today)
            .await
            .unwrap_or_else(|_| parse::parse_transcript(&req.transcript, today))
    } else {
        parse::parse_transcript(&req.transcript, today)
    };

    let response = match action {
        ParsedAction::Nutrition(new) => {
            let log = db::insert_nutrition_log(&state.pool, user.user_id, &new).await?;
            IntentResponse::logged_nutrition(
                log.id,
                format!(
                    "Logged meal: {} protein, {} carbs, {} fat.",
                    log.macros.protein.get(),
                    log.macros.carbs.get(),
                    log.macros.fat.get()
                ),
            )
        }
        ParsedAction::Workout(new) => {
            let session = db::insert_session(&state.pool, user.user_id, &new).await?;
            let summary = session.exercises.first().map_or_else(
                || "Workout logged.".to_string(),
                |ex| {
                    let set = &ex.sets[0];
                    format!(
                        "Logged {} — {} reps{}.",
                        ex.name.as_str(),
                        set.reps.get(),
                        set.weight_kg
                            .map(|w| format!(" at {} kg", w.get()))
                            .unwrap_or_default()
                    )
                },
            );
            IntentResponse::logged_workout(session.id, summary)
        }
        ParsedAction::Response(r) => r,
    };

    Ok(Json(response))
}
