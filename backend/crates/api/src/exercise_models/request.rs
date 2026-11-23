//! SPEC-0066 §2.4 — the range request: SPEC-0059's height, mass and frames,
//! and the `body` and `values` the library's one validation checks.

use std::fmt;
use std::marker::PhantomData;

use fitai_core::{
    biomech::{scale, Body, FrameCount},
    exercise::{validate, BodyMeasures, Description, Value, Values},
};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::biomech::{frame_count, height, mass};
use crate::error::{ApiError, ApiResult};

/// The wire request. Its `Debug` prints only which optional fields are
/// present, never a value (SPEC-0059 AC8).
#[derive(Deserialize)]
pub(crate) struct RangeRequest {
    height_m: f64,
    mass_kg: f64,
    // `Option`, so an explicit `null` is an absent field, as SPEC-0059.
    #[serde(default)]
    body: Option<Pairs<f64>>,
    #[serde(default)]
    values: Option<Pairs<WireValue>>,
    frames: Option<i64>,
}

impl fmt::Debug for RangeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RangeRequest")
            .field("body", &self.body.is_some())
            .field("values", &self.values.is_some())
            .field("frames", &self.frames.is_some())
            .finish_non_exhaustive()
    }
}

/// A JSON object read as its pairs, in order and with repeats kept, so the
/// validation sees a repeated key instead of the last one silently winning.
struct Pairs<V>(Vec<(String, V)>);

impl<V> Default for Pairs<V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Pairs<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor<V>(PhantomData<V>);

        impl<'de, V: Deserialize<'de>> Visitor<'de> for PairsVisitor<V> {
            type Value = Pairs<V>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Pairs(pairs))
            }
        }

        deserializer.deserialize_map(PairsVisitor(PhantomData))
    }
}

/// A `values` entry: a number, or one option of a choice by name.
#[derive(Deserialize)]
#[serde(untagged)]
enum WireValue {
    Number(f64),
    Choice(String),
}

impl From<WireValue> for Value {
    fn from(v: WireValue) -> Self {
        match v {
            WireValue::Number(n) => Self::Number(n),
            WireValue::Choice(c) => Self::Choice(c),
        }
    }
}

/// A validated request: what the model runs on. No `Debug`, since it holds
/// the lifter's measurements.
pub(crate) struct ModelInput {
    /// The table scaled by height and mass, measured lengths overriding.
    pub(crate) body: Body,
    /// The body measures the request gave.
    pub(crate) measures: BodyMeasures,
    /// Every parameter, defaults filled in.
    pub(crate) settings: Values,
    /// How many frames to sample.
    pub(crate) frames: FrameCount,
}

impl RangeRequest {
    /// Checks height and mass (SPEC-0059 §2.2), then `values` and `body`
    /// against `d` (SPEC-0066 §2.3), then `frames`.
    ///
    /// # Errors
    /// [`ApiError::Validation`] naming the first field that fails.
    pub(crate) fn validate(self, d: &Description) -> ApiResult<ModelInput> {
        let height = height(self.height_m)?;
        let mass_kg = mass(self.mass_kg)?;
        let values: Vec<(String, Value)> = self
            .values
            .unwrap_or_default()
            .0
            .into_iter()
            .map(|(key, v)| (key, v.into()))
            .collect();
        let valid = validate(d, &values, &self.body.unwrap_or_default().0).map_err(|invalid| {
            ApiError::Validation {
                field: invalid.field,
            }
        })?;
        let frames = frame_count(self.frames)?;
        Ok(ModelInput {
            body: scale(height, mass_kg, &valid.body.lengths()),
            measures: valid.body,
            settings: valid.settings,
            frames,
        })
    }
}
