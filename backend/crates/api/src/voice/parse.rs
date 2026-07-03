//! Keyword + regex voice intent parser — CI-safe fallback when no LLM key is set.
//! Mirrors the mobile keyword parser and adds workout set extraction.

use chrono::NaiveDate;
use fitai_core::{NewExercise, NewNutritionLog, NewSet, NewWorkoutSession};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    LoggedNutrition,
    LoggedWorkout,
    Clarify,
    Navigate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentResponse {
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
    pub fn clarify(prompt: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::Clarify,
            message: None,
            prompt: Some(prompt.into()),
            route: None,
            record_id: None,
        }
    }

    pub fn navigate(route: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::Navigate,
            message: Some(message.into()),
            prompt: None,
            route: Some(route.into()),
            record_id: None,
        }
    }

    pub fn logged_nutrition(id: uuid::Uuid, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::LoggedNutrition,
            message: Some(message.into()),
            prompt: None,
            route: None,
            record_id: Some(id.to_string()),
        }
    }

    pub fn logged_workout(id: uuid::Uuid, message: impl Into<String>) -> Self {
        Self {
            status: IntentStatus::LoggedWorkout,
            message: Some(message.into()),
            prompt: None,
            route: None,
            record_id: Some(id.to_string()),
        }
    }

    pub fn unknown(message: impl Into<String>) -> Self {
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
pub enum ParsedAction {
    Nutrition(NewNutritionLog),
    Workout(NewWorkoutSession),
    Response(IntentResponse),
}

pub fn parse_transcript(transcript: &str, today: NaiveDate) -> ParsedAction {
    let t = transcript.to_lowercase();
    let t = t.trim();
    if t.is_empty() {
        return ParsedAction::Response(IntentResponse::unknown(
            "Didn't catch that — try 'log a meal' or 'log 10 reps bench at 100 kg'.",
        ));
    }

    if matches_any(t, &["stop", "cancel", "pause", "never mind", "stand by", "out"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/home",
            "Standing by.",
        ));
    }

    if let Some(workout) = parse_workout_set(t, today) {
        return ParsedAction::Workout(workout);
    }

    if matches_any(
        t,
        &[
            "meal", "food", "eat", "ate", "lunch", "dinner", "breakfast", "nutrition", "macro",
        ],
    ) {
        let p = grams(t, "protein");
        let c = grams(t, "carb");
        let f = grams(t, "fat");
        if let (Some(p), Some(c), Some(f)) = (p, c, f) {
            match NewNutritionLog::new(today, p, c, f, today) {
                Ok(log) => return ParsedAction::Nutrition(log),
                Err(e) => {
                    return ParsedAction::Response(IntentResponse::clarify(format!(
                        "Those macros didn't validate ({}) — try again.",
                        e.field()
                    )));
                }
            }
        }
        return ParsedAction::Response(IntentResponse::clarify(
            "Tell me protein, carbs, and fat in grams — for example: 40 protein, 60 carbs, 20 fat.",
        ));
    }

    if matches_any(t, &["plan"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/current",
            "Opening your program.",
        ));
    }
    if matches_any(
        t,
        &["workout", "session", "train", "exercise", "gym", "lift", "start workout"],
    ) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/session",
            "Starting your session.",
        ));
    }
    if matches_any(t, &["body type", "body match", "match me", "find my type"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/get",
            "Opening body match.",
        ));
    }
    if matches_any(t, &["program", "routine"]) {
        return ParsedAction::Response(IntentResponse::navigate(
            "/programs/current",
            "Opening your program.",
        ));
    }
    if matches_any(t, &["history", "past", "log list", "sessions"]) {
        return ParsedAction::Response(IntentResponse::navigate("/home", "Your recent activity."));
    }
    if matches_any(t, &["profile", "my details", "settings", "account"]) {
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
    let cleaned = Regex::new(r"\d+(?:\.\d+)?|\b(?:kg|kilos?|kilo|reps?|of|at|the|a|an|i|did|log|logged)\b")
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

pub async fn parse_with_llm(
    client: &reqwest::Client,
    api_key: &str,
    transcript: &str,
    today: NaiveDate,
) -> ApiResult<ParsedAction> {
    let prompt = format!(
        "You parse gym voice commands into JSON only. Today is {today}. \
         Transcript: \"{transcript}\"\n\
         Return ONE of:\n\
         {{\"action\":\"log_workout\",\"exercise\":\"name\",\"reps\":N,\"weight_kg\":N|null}}\n\
         {{\"action\":\"log_meal\",\"protein_g\":N,\"carbs_g\":N,\"fat_g\":N}}\n\
         {{\"action\":\"clarify\",\"prompt\":\"question\"}}\n\
         {{\"action\":\"navigate\",\"route\":\"/session|/home|/programs/current|/programs/get|/onboarding\",\"message\":\"...\"}}\n\
         {{\"action\":\"unknown\",\"message\":\"...\"}}"
    );
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": prompt}]
    });
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
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
    let text = json["content"][0]["text"].as_str().ok_or(ApiError::Upstream)?;
    let parsed: serde_json::Value = serde_json::from_str(text.trim())
        .or_else(|_| extract_json_object(text))
        .map_err(|_| ApiError::Upstream)?;
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
            let reps = v["reps"].as_i64().ok_or(ApiError::Upstream)? as i32;
            let weight = v["weight_kg"].as_f64();
            let set = NewSet::new(reps, weight, None).map_err(|_| ApiError::Upstream)?;
            let ex = NewExercise::new(exercise, None, vec![set]).map_err(|_| ApiError::Upstream)?;
            let session = NewWorkoutSession::new(today, vec![ex], today).map_err(|_| ApiError::Upstream)?;
            Ok(ParsedAction::Workout(session))
        }
        Some("log_meal") => {
            let p = v["protein_g"].as_f64().ok_or(ApiError::Upstream)?;
            let c = v["carbs_g"].as_f64().ok_or(ApiError::Upstream)?;
            let f = v["fat_g"].as_f64().ok_or(ApiError::Upstream)?;
            let log = NewNutritionLog::new(today, p, c, f, today).map_err(|e| ApiError::Validation {
                field: e.field(),
            })?;
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
}
