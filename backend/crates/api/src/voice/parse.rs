//! Keyword + regex voice intent parser — CI-safe fallback when no LLM key is set.
//! Mirrors the mobile keyword parser and adds workout set extraction.

use chrono::NaiveDate;
use fitai_core::{NewExercise, NewNutritionLog, NewSet, NewWorkoutSession};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum IntentStatus {
    LoggedNutrition,
    LoggedWorkout,
    Clarify,
    Navigate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super) struct IntentResponse {
    pub status: IntentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
}

impl IntentResponse {
    pub(super) fn clarify(prompt: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::Clarify,
            message: None,
            prompt: Some(prompt.into()),
            route: None,
            record_id: None,
        }
    }

    pub(super) fn navigate(route: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::Navigate,
            message: Some(message.into()),
            prompt: None,
            route: Some(route.into()),
            record_id: None,
        }
    }

    pub(super) fn logged_nutrition(id: uuid::Uuid, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::LoggedNutrition,
            message: Some(message.into()),
            prompt: None,
            route: None,
            record_id: Some(id.to_string()),
        }
    }

    pub(super) fn logged_workout(id: uuid::Uuid, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::LoggedWorkout,
            message: Some(message.into()),
            prompt: None,
            route: None,
            record_id: Some(id.to_string()),
        }
    }

    pub(super) fn unknown(message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::Unknown,
            message: Some(message.into()),
            prompt: None,
            route: None,
            record_id: None,
        }
    }
}

/// Parsed write models ready for persistence.
pub(super) enum ParsedAction {
    Nutrition(NewNutritionLog),
    Workout(NewWorkoutSession),
    Response(IntentResponse),
}

pub(super) fn parse_transcript(transcript: &str, today: NaiveDate) -> ParsedAction {
    let text = transcript.to_lowercase();
    let text = text.trim();
    if text.is_empty() {
        return ParsedAction::Response(IntentResponse::unknown(
            "Didn't catch that — try 'log a meal' or 'log 10 reps bench at 100 kg'.",
        ));
    }

    if matches_any(
        text,
        &["stop", "cancel", "pause", "never mind", "stand by", "out"],
    ) {
        return ParsedAction::Response(IntentResponse::navigate("/home", "Standing by."));
    }

    if let Some(workout) = parse_workout_set(text, today) {
        return ParsedAction::Workout(workout);
    }

    if matches_any(
        text,
        &[
            "meal",
            "food",
            "eat",
            "ate",
            "lunch",
            "dinner",
            "breakfast",
            "nutrition",
            "macro",
        ],
    ) {
        let protein_g = grams(text, "protein");
        let carbs_g = grams(text, "carb");
        let fat_g = grams(text, "fat");
        if let (Some(protein_g), Some(carbs_g), Some(fat_g)) = (protein_g, carbs_g, fat_g) {
            match NewNutritionLog::new(today, protein_g, carbs_g, fat_g, today) {
                Ok(log) => return ParsedAction::Nutrition(log),
                Err(err) => {
                    return ParsedAction::Response(IntentResponse::clarify(format!(
                        "Those macros didn't validate ({}) — try again.",
                        err.field()
                    )));
                }
            }
        }
        return ParsedAction::Response(IntentResponse::clarify(
            "Tell me protein, carbs, and fat in grams — for example: 40 protein, 60 carbs, 20 fat.",
        ));
    }

    if matches_any(text, &["plan"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/current",
            "Opening your program.",
        ));
    }
    if matches_any(
        text,
        &[
            "workout",
            "session",
            "train",
            "exercise",
            "gym",
            "lift",
            "start workout",
        ],
    ) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/session",
            "Starting your session.",
        ));
    }
    if matches_any(
        text,
        &["body type", "body match", "match me", "find my type"],
    ) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/get",
            "Opening body match.",
        ));
    }
    if matches_any(text, &["program", "routine"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/current",
            "Opening your program.",
        ));
    }
    if matches_any(text, &["history", "past", "log list", "sessions"]) {
        return ParsedAction::Response(IntentResponse::navigate("/home", "Your recent activity."));
    }
    if matches_any(text, &["profile", "my details", "settings", "account"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/onboarding",
            "Opening your profile.",
        ));
    }

    ParsedAction::Response(IntentResponse::unknown(format!(
        "Didn't understand \"{transcript}\" — try 'log a meal' or 'log 10 reps bench at 100 kg'."
    )))
}

fn parse_workout_set(t: &str, today: NaiveDate) -> Option<NewWorkoutSession> {
    // "10 reps of 100 kg bench press" / "bench press 10 reps 100 kg"
    let reps_re = Regex::new(r"(\d+)\s*reps?").ok()?;
    let weight_re = Regex::new(r"(\d+(?:\.\d+)?)\s*(?:kg|kilos?|kilo)").ok()?;
    let reps = reps_re
        .captures(t)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())?;
    let weight = weight_re
        .captures(t)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());
    let exercise = extract_exercise_name(t)?;
    let set = NewSet::new(reps, weight, None).ok()?;
    let ex = NewExercise::new(&exercise, None, vec![set]).ok()?;
    NewWorkoutSession::new(today, vec![ex], today).ok()
}

fn extract_exercise_name(t: &str) -> Option<String> {
    let bench = Regex::new(r"\bbench\s*press\b").ok()?;
    if bench.is_match(t) {
        return Some("Bench press".to_string());
    }
    let squat = Regex::new(r"\bsquat\b").ok()?;
    if squat.is_match(t) {
        return Some("Squat".to_string());
    }
    let dead = Regex::new(r"\bdead\s*lift\b").ok()?;
    if dead.is_match(t) {
        return Some("Deadlift".to_string());
    }
    // Fallback: strip numbers/units and use remainder if long enough.
    let cleaned =
        Regex::new(r"\d+(?:\.\d+)?|\b(?:kg|kilos?|kilo|reps?|of|at|the|a|an|i|did|log|logged)\b")
            .ok()?
            .replace_all(t, " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    if cleaned.len() >= 3 {
        Some(cleaned)
    } else {
        None
    }
}

fn matches_any(t: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| t.contains(k))
}

fn grams(t: &str, macro_name: &str) -> Option<f64> {
    let before = Regex::new(&format!(
        r"(\d+(?:\.\d+)?)\s*(?:g|grams?)?\s*(?:of\s+)?{macro_name}"
    ))
    .ok()?;
    let after = Regex::new(&format!(r"{macro_name}[a-z]*\s+(\d+(?:\.\d+)?)")).ok()?;
    before
        .captures(t)
        .or_else(|| after.captures(t))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Which wire protocol the configured LLM endpoint speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LlmProvider {
    /// Anthropic Messages API (`/v1/messages`, `x-api-key`, `content[0].text`).
    Anthropic,
    /// OpenAI-compatible chat API (`/chat/completions`, `Bearer`,
    /// `choices[0].message.content`) — this is what a local Ollama serves.
    OpenAiCompatible,
}

/// Resolved LLM endpoint. Built from env so local dev can point voice-intent
/// parsing at Ollama (or any OpenAI-compatible server) instead of Anthropic,
/// with the keyword parser still the fallback when no LLM is configured.
#[derive(Debug, Clone)]
pub(super) struct LlmConfig {
    provider: LlmProvider,
    /// Root URL without the endpoint path (e.g. `http://localhost:11434/v1`).
    base_url: String,
    model: String,
    /// May be empty (Ollama needs no key).
    api_key: String,
}

impl LlmConfig {
    /// Pure resolver — no process env, so it is unit-testable. Returns `None`
    /// when the LLM path should be skipped (Anthropic selected but no key),
    /// which makes the caller fall back to the keyword parser.
    fn resolve(
        provider: Option<&str>,
        base: Option<String>,
        model: Option<String>,
        key: Option<String>,
    ) -> Option<Self> {
        let key = key.filter(|k| !k.is_empty());
        match provider.unwrap_or("anthropic") {
            "anthropic" => Some(Self {
                provider: LlmProvider::Anthropic,
                base_url: base.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                model: model.unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string()),
                // Anthropic requires a key; without one there is no LLM path.
                api_key: key?,
            }),
            // "ollama" | "openai" | "openai-compatible" | anything else.
            _ => Some(Self {
                provider: LlmProvider::OpenAiCompatible,
                base_url: base.unwrap_or_else(|| "http://localhost:11434/v1".to_string()),
                model: model.unwrap_or_else(|| "llama3.2".to_string()),
                api_key: key.unwrap_or_default(),
            }),
        }
    }

    /// Reads `LLM_PROVIDER` / `LLM_BASE_URL` / `LLM_MODEL` / `LLM_API_KEY`
    /// (falling back to `ANTHROPIC_API_KEY` for the key). Returns `None` when
    /// no LLM is configured, so voice parsing uses the keyword fallback.
    pub(super) fn from_env() -> Option<Self> {
        let provider = std::env::var("LLM_PROVIDER").ok();
        Self::resolve(
            provider.as_deref(),
            std::env::var("LLM_BASE_URL").ok(),
            std::env::var("LLM_MODEL").ok(),
            std::env::var("LLM_API_KEY")
                .ok()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()),
        )
    }

    fn timeout(&self) -> std::time::Duration {
        // A local model can be slow on the first (model-load) request.
        let secs = match self.provider {
            LlmProvider::Anthropic => 8,
            LlmProvider::OpenAiCompatible => 60,
        };
        std::time::Duration::from_secs(secs)
    }
}

/// Extracts the assistant's text from a provider-specific response body.
fn extract_llm_text(provider: LlmProvider, json: &serde_json::Value) -> Option<String> {
    let field = match provider {
        LlmProvider::Anthropic => &json["content"][0]["text"],
        LlmProvider::OpenAiCompatible => &json["choices"][0]["message"]["content"],
    };
    field.as_str().map(str::to_string)
}

const LLM_PROMPT_HEAD: &str = "You parse gym voice commands into JSON only.";

pub(super) async fn parse_with_llm(
    client: &reqwest::Client,
    cfg: &LlmConfig,
    transcript: &str,
    today: NaiveDate,
) -> ApiResult<ParsedAction> {
    let prompt = format!(
        "{LLM_PROMPT_HEAD} Today is {today}. \
         Transcript: \"{transcript}\"\n\
         Return ONE of:\n\
         {{\"action\":\"log_workout\",\"exercise\":\"name\",\"reps\":N,\"weight_kg\":N|null}}\n\
         {{\"action\":\"log_meal\",\"protein_g\":N,\"carbs_g\":N,\"fat_g\":N}}\n\
         {{\"action\":\"clarify\",\"prompt\":\"question\"}}\n\
         {{\"action\":\"navigate\",\"route\":\"/session|/home|/programs/current|/programs/get|/onboarding\",\"message\":\"...\"}}\n\
         {{\"action\":\"unknown\",\"message\":\"...\"}}"
    );

    let (body, req) = match cfg.provider {
        LlmProvider::Anthropic => {
            let body = serde_json::json!({
                "model": cfg.model,
                "max_tokens": 256,
                "messages": [{"role": "user", "content": prompt}]
            });
            let req = client
                .post(format!("{}/v1/messages", cfg.base_url))
                .header("x-api-key", &cfg.api_key)
                .header("anthropic-version", "2023-06-01");
            (body, req)
        }
        LlmProvider::OpenAiCompatible => {
            let body = serde_json::json!({
                "model": cfg.model,
                "max_tokens": 256,
                "stream": false,
                "response_format": {"type": "json_object"},
                "messages": [{"role": "user", "content": prompt}]
            });
            let mut req = client.post(format!("{}/chat/completions", cfg.base_url));
            if !cfg.api_key.is_empty() {
                req = req.header("authorization", format!("Bearer {}", cfg.api_key));
            }
            (body, req)
        }
    };

    let resp = req
        .timeout(cfg.timeout())
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|_| ApiError::Upstream)?;
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ApiError::Upstream);
    }
    if !resp.status().is_success() {
        return Err(ApiError::Upstream);
    }
    let body_text = resp.text().await.map_err(|_| ApiError::Upstream)?;
    let json: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|_| ApiError::Upstream)?;
    let text = extract_llm_text(cfg.provider, &json).ok_or(ApiError::Upstream)?;
    let parsed: serde_json::Value = serde_json::from_str(text.trim())
        .or_else(|_| extract_json_object(&text))
        .map_err(|()| ApiError::Upstream)?;
    llm_json_to_action(&parsed, today)
}

fn extract_json_object(text: &str) -> Result<serde_json::Value, ()> {
    let start = text.find('{').ok_or(())?;
    let end = text.rfind('}').ok_or(())?;
    serde_json::from_str(&text[start..=end]).map_err(|_| ())
}

fn llm_json_to_action(v: &serde_json::Value, today: NaiveDate) -> ApiResult<ParsedAction> {
    match v["action"].as_str() {
        Some("log_workout") => {
            let exercise = v["exercise"].as_str().unwrap_or("Exercise");
            let reps = i32::try_from(v["reps"].as_i64().ok_or(ApiError::Upstream)?)
                .map_err(|_| ApiError::Upstream)?;
            let weight = v["weight_kg"].as_f64();
            let set = NewSet::new(reps, weight, None).map_err(|_| ApiError::Upstream)?;
            let ex = NewExercise::new(exercise, None, vec![set]).map_err(|_| ApiError::Upstream)?;
            let session =
                NewWorkoutSession::new(today, vec![ex], today).map_err(|_| ApiError::Upstream)?;
            Ok(ParsedAction::Workout(session))
        }
        Some("log_meal") => {
            let p = v["protein_g"].as_f64().ok_or(ApiError::Upstream)?;
            let c = v["carbs_g"].as_f64().ok_or(ApiError::Upstream)?;
            let f = v["fat_g"].as_f64().ok_or(ApiError::Upstream)?;
            let log = NewNutritionLog::new(today, p, c, f, today)
                .map_err(|e| ApiError::Validation { field: e.field() })?;
            Ok(ParsedAction::Nutrition(log))
        }
        Some("clarify") => Ok(ParsedAction::Response(IntentResponse::clarify(
            v["prompt"].as_str().unwrap_or("Can you say that again?"),
        ))),
        Some("navigate") => Ok(ParsedAction::Response(IntentResponse::navigate(
            v["route"].as_str().unwrap_or("/home"),
            v["message"].as_str().unwrap_or("Roger."),
        ))),
        _ => Ok(ParsedAction::Response(IntentResponse::unknown(
            v["message"].as_str().unwrap_or("Didn't understand that."),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bench_set_from_natural_language() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let action = parse_transcript("I did 10 reps of 100 kg bench press", today);
        match action {
            ParsedAction::Workout(session) => {
                assert_eq!(session.exercises.len(), 1);
                assert_eq!(session.exercises[0].name.as_str(), "Bench press");
                assert_eq!(session.exercises[0].sets[0].reps.get(), 10);
            }
            _ => panic!("expected workout"),
        }
    }

    #[test]
    fn meal_without_macros_clarifies() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        match parse_transcript("log a meal", today) {
            ParsedAction::Response(r) => assert_eq!(r.status, IntentStatus::Clarify),
            _ => panic!("expected clarify"),
        }
    }

    #[test]
    fn resolve_defaults_to_anthropic_when_key_present() {
        let cfg = LlmConfig::resolve(None, None, None, Some("sk-ant".into())).unwrap();
        assert_eq!(cfg.provider, LlmProvider::Anthropic);
        assert_eq!(cfg.base_url, "https://api.anthropic.com");
        assert_eq!(cfg.model, "claude-haiku-4-5-20251001");
        assert_eq!(cfg.api_key, "sk-ant");
    }

    #[test]
    fn resolve_anthropic_without_key_is_none() {
        assert!(LlmConfig::resolve(Some("anthropic"), None, None, None).is_none());
        // An empty key is treated as absent.
        assert!(LlmConfig::resolve(Some("anthropic"), None, None, Some(String::new())).is_none());
    }

    #[test]
    fn resolve_ollama_needs_no_key_and_defaults_to_localhost() {
        let cfg = LlmConfig::resolve(Some("ollama"), None, None, None).unwrap();
        assert_eq!(cfg.provider, LlmProvider::OpenAiCompatible);
        assert_eq!(cfg.base_url, "http://localhost:11434/v1");
        assert_eq!(cfg.model, "llama3.2");
        assert!(cfg.api_key.is_empty());
    }

    #[test]
    fn resolve_openai_compatible_honours_custom_base_and_model() {
        let cfg = LlmConfig::resolve(
            Some("openai-compatible"),
            Some("http://gpu-box:8000/v1".into()),
            Some("qwen3.5:9b".into()),
            Some("token".into()),
        )
        .unwrap();
        assert_eq!(cfg.provider, LlmProvider::OpenAiCompatible);
        assert_eq!(cfg.base_url, "http://gpu-box:8000/v1");
        assert_eq!(cfg.model, "qwen3.5:9b");
        assert_eq!(cfg.api_key, "token");
    }

    #[test]
    fn extract_text_reads_each_provider_shape() {
        let anthropic = serde_json::json!({"content": [{"text": "hi from claude"}]});
        assert_eq!(
            extract_llm_text(LlmProvider::Anthropic, &anthropic).as_deref(),
            Some("hi from claude")
        );
        let openai = serde_json::json!({"choices": [{"message": {"content": "hi from ollama"}}]});
        assert_eq!(
            extract_llm_text(LlmProvider::OpenAiCompatible, &openai).as_deref(),
            Some("hi from ollama")
        );
        // Wrong shape → None (caller falls back).
        assert!(extract_llm_text(LlmProvider::Anthropic, &openai).is_none());
    }
}
