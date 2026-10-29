//! SPEC-0059 §2.2 — the squat range request and its validation table.
//!
//! `scale` and `MeasuredLengths` validate nothing, so these are the only
//! guards between the wire and the engine. Height and mass use the profile's
//! own ranges, so a value the profile accepted is never a `400` here.

use std::fmt;

use fitai_core::{
    biomech::{scale, Body, Dorsiflexion, FrameCount, MeasuredLengths, Metres},
    profile::{HeightCm, WeightKg},
    workout::LoadKg,
};
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};

/// The frame count when the request leaves it out.
const DEFAULT_FRAMES: i64 = 41;

/// Bounds on a measured segment length, metres.
const LENGTH_M: std::ops::RangeInclusive<f64> = 0.1..=1.5;

/// The wire request. Its `Debug` prints only which optional fields are
/// present, never a measurement (AC8).
#[derive(Deserialize)]
pub(crate) struct RangeRequest {
    height_m: f64,
    mass_kg: f64,
    thigh_m: Option<f64>,
    shank_m: Option<f64>,
    trunk_m: Option<f64>,
    dorsiflexion_deg: f64,
    bar_kg: Option<f64>,
    frames: Option<i64>,
}

impl fmt::Debug for RangeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RangeRequest")
            .field("thigh_m", &self.thigh_m.is_some())
            .field("shank_m", &self.shank_m.is_some())
            .field("trunk_m", &self.trunk_m.is_some())
            .field("bar_kg", &self.bar_kg.is_some())
            .field("frames", &self.frames.is_some())
            .finish_non_exhaustive()
    }
}

/// A validated request: the engine's inputs, plus what the envelope reports.
/// No `Debug`, since it holds the lifter's measurements.
pub(crate) struct Squat {
    /// The lifter: the table scaled by height and mass, measured lengths overriding.
    pub(crate) body: Body,
    /// Body mass, kilograms, for the ground force.
    pub(crate) mass_kg: f64,
    /// The bar's load; `None` is bodyweight only.
    pub(crate) bar: Option<LoadKg>,
    /// The lifter's ankle dorsiflexion.
    pub(crate) ankle: Dorsiflexion,
    /// How many frames to sample across the range.
    pub(crate) frames: FrameCount,
    /// Which of `thigh`, `shank`, `trunk` the request left to the table (AC2).
    pub(crate) table_lengths: Vec<&'static str>,
}

impl RangeRequest {
    /// Check every field against SPEC-0059 §2.2's table, in the table's order.
    ///
    /// # Errors
    /// [`ApiError::Validation`] naming the first field out of range.
    pub(crate) fn validate(self) -> ApiResult<Squat> {
        let height = within(
            self.height_m,
            f64::from(HeightCm::MIN) / 100.0..=f64::from(HeightCm::MAX) / 100.0,
            "height_m",
        )?;
        let mass_kg = within(self.mass_kg, WeightKg::MIN..=WeightKg::MAX, "mass_kg")?;
        let measured = MeasuredLengths {
            thigh: length(self.thigh_m, "thigh_m")?,
            shank: length(self.shank_m, "shank_m")?,
            trunk: length(self.trunk_m, "trunk_m")?,
        };
        let ankle =
            Dorsiflexion::from_degrees(self.dorsiflexion_deg).ok_or(ApiError::Validation {
                field: "dorsiflexion_deg",
            })?;
        let bar = self
            .bar_kg
            .map(LoadKg::try_new)
            .transpose()
            .map_err(|_| ApiError::Validation { field: "bar_kg" })?;
        let frames = u16::try_from(self.frames.unwrap_or(DEFAULT_FRAMES))
            .ok()
            .and_then(FrameCount::new)
            .ok_or(ApiError::Validation { field: "frames" })?;
        let table_lengths = [
            ("thigh", measured.thigh),
            ("shank", measured.shank),
            ("trunk", measured.trunk),
        ]
        .into_iter()
        .filter_map(|(name, length)| length.is_none().then_some(name))
        .collect();
        Ok(Squat {
            body: scale(Metres(height), mass_kg, &measured),
            mass_kg,
            bar,
            ankle,
            frames,
            table_lengths,
        })
    }
}

fn within(value: f64, range: std::ops::RangeInclusive<f64>, field: &'static str) -> ApiResult<f64> {
    if range.contains(&value) {
        Ok(value)
    } else {
        Err(ApiError::Validation { field })
    }
}

fn length(value: Option<f64>, field: &'static str) -> ApiResult<Option<Metres>> {
    value
        .map(|m| within(m, LENGTH_M, field).map(Metres))
        .transpose()
}
