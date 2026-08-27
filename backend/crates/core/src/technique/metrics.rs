//! The measured quantities and their exact user-facing copy (SPEC-0044 §2.7.6,
//! AC11/AC14).
//!
//! Enumerated in full — with the strings in the source rather than in a UI
//! someone writes later — so that "no medical or injury claims" is
//! **mechanically reviewable**. The suite asserts every string here and scans
//! them for clinical and injury vocabulary.

use serde::{Deserialize, Serialize};

/// A quantity the analysis measures from the keypoint series.
///
/// **Named `Metric`, not `Fault`** (architect review finding 25): a clean squat
/// still produces a `SquatDepth` value, and a depth of −5° is not a fault. The
/// type names *what was measured*; [`FindingSeverity`](super::FindingSeverity)
/// — where a threshold exists at all — says whether it passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// Hip→knee angle from horizontal at the bottom of the rep.
    SquatDepth,
    /// Knee displacement toward the midline, front view, per side.
    KneeTravelInward,
    /// Change in torso inclination through a squat rep.
    SquatTorsoAngleChange,
    /// Horizontal travel of the bar proxy through a bench rep.
    BenchBarPathDeviation,
    /// Forearm angle from vertical at the chest.
    BenchForearmAngleAtTouch,
    /// Spread of the touch point across the reps of a set.
    BenchTouchPointConsistency,
    /// How much the hips rise before the bar breaks the floor.
    DeadliftHipRiseBeforeBar,
    /// Change in torso inclination through the pull.
    DeadliftTorsoAngleChange,
    /// Horizontal distance of the bar proxy from the ankle.
    DeadliftBarDriftFromAnkle,
}

impl Metric {
    /// Every metric, for exhaustive iteration in tests and in the UI.
    pub const ALL: [Metric; 9] = [
        Metric::SquatDepth,
        Metric::KneeTravelInward,
        Metric::SquatTorsoAngleChange,
        Metric::BenchBarPathDeviation,
        Metric::BenchForearmAngleAtTouch,
        Metric::BenchTouchPointConsistency,
        Metric::DeadliftHipRiseBeforeBar,
        Metric::DeadliftTorsoAngleChange,
        Metric::DeadliftBarDriftFromAnkle,
    ];

    /// The metric's user-facing name. Describes a movement, never a diagnosis.
    ///
    /// "Valgus" is deliberately absent from [`Metric::KneeTravelInward`]: it is
    /// clinical vocabulary and a lifter reads it as a diagnosis (AC14).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Metric::SquatDepth => "Depth at the bottom",
            Metric::KneeTravelInward => "Knee travel (inward)",
            Metric::SquatTorsoAngleChange => "Torso angle change",
            Metric::BenchBarPathDeviation => "Bar path (horizontal travel)",
            Metric::BenchForearmAngleAtTouch => "Forearm angle at the chest",
            Metric::BenchTouchPointConsistency => "Touch point consistency",
            Metric::DeadliftHipRiseBeforeBar => "Hip rise before the bar moves",
            Metric::DeadliftTorsoAngleChange => "Torso angle change during the pull",
            Metric::DeadliftBarDriftFromAnkle => "Bar distance from the ankle",
        }
    }

    /// The coaching suggestion shown beside the value. A suggestion about
    /// movement — never a prediction about the body (AC14).
    #[must_use]
    pub fn cue(self) -> &'static str {
        match self {
            Metric::SquatDepth => "Aim to bring the hip crease level with the top of the knee.",
            Metric::KneeTravelInward => {
                "Think about spreading the floor with your feet so the knees track over the toes."
            }
            Metric::SquatTorsoAngleChange => {
                "Try to hold the torso angle you start the rep with, all the way down and up."
            }
            Metric::BenchBarPathDeviation => "Aim for the bar to travel the same line down and up.",
            Metric::BenchForearmAngleAtTouch => {
                "At the chest, aim to have the forearm vertical under the bar."
            }
            Metric::BenchTouchPointConsistency => "Aim to touch the same spot on the chest each rep.",
            Metric::DeadliftHipRiseBeforeBar => {
                "Try to start the hips and the bar together rather than letting the hips rise first."
            }
            Metric::DeadliftTorsoAngleChange => {
                "Aim to hold the torso angle you set up with until the bar passes the knee."
            }
            Metric::DeadliftBarDriftFromAnkle => "Aim to keep the bar close to the leg through the pull.",
        }
    }
}
