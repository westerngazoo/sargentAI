//! Food nutrient lookup — `GET /nutrition/foods?q=…` proxies USDA
//! `FoodData Central` so the client can turn "200 grams of chicken breast"
//! into macros. The key comes from `FDC_API_KEY` (falls back to the
//! rate-limited `DEMO_KEY`); the upstream JSON is reduced to per-100 g
//! macros by a pure, unit-tested parser.

use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};

const FDC_SEARCH_URL: &str = "https://api.nal.usda.gov/fdc/v1/foods/search";
const NUTRIENT_PROTEIN: i64 = 1003;
const NUTRIENT_FAT: i64 = 1004;
const NUTRIENT_CARBS: i64 = 1005;
const NUTRIENT_KCAL: i64 = 1008;

#[derive(Debug, Deserialize)]
pub(crate) struct FoodQuery {
    q: String,
}

/// One food with macros per 100 g, as served to the client.
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct FoodMacros {
    pub name: String,
    pub protein_g_per_100g: f64,
    pub carbs_g_per_100g: f64,
    pub fat_g_per_100g: f64,
    pub kcal_per_100g: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct FoodSearchResponse {
    pub foods: Vec<FoodMacros>,
}

pub(crate) async fn search(
    _user: AuthenticatedUser,
    Query(query): Query<FoodQuery>,
) -> ApiResult<Json<FoodSearchResponse>> {
    let q = query.q.trim();
    if q.is_empty() {
        return Err(ApiError::Validation { field: "q" });
    }
    let key = std::env::var("FDC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| ApiError::Internal(eyre::eyre!(e)))?;
    let body = client
        .get(FDC_SEARCH_URL)
        .query(&[
            ("api_key", key.as_str()),
            ("query", q),
            ("dataType", "Foundation,SR Legacy"),
            ("pageSize", "5"),
        ])
        .send()
        .await
        .map_err(|_| ApiError::Upstream)?
        .text()
        .await
        .map_err(|_| ApiError::Upstream)?;
    let foods = parse_fdc_response(&body).ok_or(ApiError::Upstream)?;
    Ok(Json(FoodSearchResponse { foods }))
}

/// Reduces the FDC search payload to per-100 g macros. Returns `None` only
/// when the payload is not the expected shape (upstream error bodies).
fn parse_fdc_response(body: &str) -> Option<Vec<FoodMacros>> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let foods = value.get("foods")?.as_array()?;
    Some(
        foods
            .iter()
            .filter_map(|food| {
                let name = food.get("description")?.as_str()?.to_string();
                let nutrients = food.get("foodNutrients")?.as_array()?;
                let get = |id: i64| -> f64 {
                    nutrients
                        .iter()
                        .find(|n| {
                            n.get("nutrientId").and_then(serde_json::Value::as_i64) == Some(id)
                        })
                        .and_then(|n| n.get("value"))
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0)
                };
                Some(FoodMacros {
                    name,
                    protein_g_per_100g: get(NUTRIENT_PROTEIN),
                    carbs_g_per_100g: get(NUTRIENT_CARBS),
                    fat_g_per_100g: get(NUTRIENT_FAT),
                    kcal_per_100g: get(NUTRIENT_KCAL),
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "foods": [{
            "description": "Chicken, breast, grilled",
            "foodNutrients": [
                {"nutrientId": 1003, "value": 31.0},
                {"nutrientId": 1004, "value": 3.5},
                {"nutrientId": 1005, "value": 0.0},
                {"nutrientId": 1008, "value": 165.0}
            ]
        }]
    }"#;

    #[test]
    fn parses_fdc_search_payload_to_per_100g_macros() {
        let foods = parse_fdc_response(FIXTURE).expect("parses");
        assert_eq!(
            foods,
            vec![FoodMacros {
                name: "Chicken, breast, grilled".into(),
                protein_g_per_100g: 31.0,
                carbs_g_per_100g: 0.0,
                fat_g_per_100g: 3.5,
                kcal_per_100g: 165.0,
            }]
        );
    }

    #[test]
    fn missing_nutrients_default_to_zero() {
        let body = r#"{"foods":[{"description":"Mystery","foodNutrients":[]}]}"#;
        let foods = parse_fdc_response(body).expect("parses");
        assert!(foods[0].protein_g_per_100g.abs() < f64::EPSILON);
        assert!(foods[0].kcal_per_100g.abs() < f64::EPSILON);
    }

    #[test]
    fn upstream_error_body_yields_none() {
        assert!(parse_fdc_response(r#"{"error":"forbidden"}"#).is_none());
        assert!(parse_fdc_response("not json").is_none());
    }
}
