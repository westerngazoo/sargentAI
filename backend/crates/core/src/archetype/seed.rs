//! The six owner-approved archetype records (R-0012 AC2).
//!
//! Each builder returns a `Result` so it can thread the validating constructors
//! with `?`; [`all`] discharges them with the single justified `expect`
//! (architect finding 1, option B). Every record is proven valid by the SAC2
//! test `library_records_each_revalidate_for_internal_consistency`, so a
//! malformed record fails the build — the panic in [`all`] is genuinely
//! unreachable (CLAUDE.md §6).
//!
//! `display_name`/`summary` and every template string are **abstracted**: the
//! internal research label and the curation `sources` (which name the athletes
//! and their books) are internal-only and never serialized (AC4). The `sources`
//! anchor each record to its primary material and flag the provenance honestly
//! (AC7).

use crate::Goal;

use super::{
    Archetype, ArchetypeError, Confidence, DietTemplate, FrameProfile, HeightBand, LengthBand,
    MacroEmphasis, ProgramTemplate, Provenance, Somatotype, StructureTag, TrainingPhilosophy,
    VolumeBand, WidthBand,
};

/// The whole curated library, in authored order. The single `expect` is
/// discharged against compile-time-constant records the SAC2 test proves valid.
#[allow(clippy::expect_used)] // see module doc: records are test-gated by SAC2;
                              // an invalid one fails the build, so this is genuinely unreachable.
pub(crate) fn all() -> Vec<Archetype> {
    vec![
        low_volume_mass(),
        minimalist_high_intensity(),
        classic_aesthetic_taper(),
        compact_powerbuilder(),
        high_volume_mass(),
        precision_hypertrophy(),
    ]
    .into_iter()
    .map(|r| r.expect("seed record must validate (SAC2)"))
    .collect()
}

/// Low-volume, high-intensity mass for a tall, thick, dense frame. (Yates-96 —
/// well-documented: athlete-authored book + filmed training.)
fn low_volume_mass() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "heavy-duty-mass",
        "Yates-96",
        "Low-Volume Mass Builder".to_string(),
        "Brief, brutally hard sessions: one all-out working set per exercise for \
         a tall, thick, dense frame. A starting point to personalize from."
            .to_string(),
        FrameProfile::new(
            1.5,
            HeightBand::Tall,
            WidthBand::Average,
            LengthBand::Average,
            Somatotype::Meso,
            vec![StructureTag::DenseMuscle, StructureTag::BlockyWaist],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::Hit,
            "4-day split (delts/triceps, back, chest/biceps, legs); each muscle \
             once every ~6 days"
                .to_string(),
            1,
            VolumeBand::Low,
            "1 all-out working set to failure after 1-2 warm-ups; forced reps and \
             negatives used sparingly"
                .to_string(),
            "2-3 min between sets; sessions capped ~45-60 min".to_string(),
            "add load once the rep target is reached".to_string(),
        )?,
        DietTemplate::new(
            "high-protein structured clean bulk".to_string(),
            "moderate surplus for lean mass".to_string(),
            MacroEmphasis::HighProtein,
            "~6 meals per day".to_string(),
        ),
        Provenance::new(
            Confidence::Documented,
            vec![
                "Dorian Yates & Bob Wolff, Blood and Guts (1993, ISBN 0963616307)",
                "Blood & Guts training video (1993, Temple Gym, Birmingham)",
            ],
        ),
        vec![Goal::BuildMuscle, Goal::GainStrength],
    )
}

/// A minimalist one-set-to-failure method for an average, thick, dense frame.
/// (Mentzer — well-documented: athlete-authored books.)
fn minimalist_high_intensity() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "high-intensity-minimalist",
        "Mentzer",
        "Minimalist High-Intensity".to_string(),
        "The fewest sets that drive growth: one set to failure per exercise with \
         long recovery between sessions. A starting point to personalize from."
            .to_string(),
        FrameProfile::new(
            1.5,
            HeightBand::Average,
            WidthBand::Average,
            LengthBand::Average,
            Somatotype::Meso,
            vec![StructureTag::DenseMuscle],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::Hit,
            "4-workout rotation (chest/back, legs, delts/arms, legs); each muscle \
             every ~4-7 days"
                .to_string(),
            1,
            VolumeBand::Low,
            "one set to failure per exercise, 6-8 reps, after warm-ups; \
             pre-exhaust supersets"
                .to_string(),
            "long inter-session recovery (4-7 days)".to_string(),
            "double progression: add reps, then load".to_string(),
        )?,
        DietTemplate::new(
            "balanced whole-food diet, no extremes".to_string(),
            "modest surplus or maintenance".to_string(),
            MacroEmphasis::Balanced,
            "3-4 meals per day".to_string(),
        ),
        Provenance::new(
            Confidence::Documented,
            vec![
                "Mike Mentzer, Heavy Duty (1993)",
                "Mike Mentzer, Heavy Duty II: Mind and Body (1996)",
            ],
        ),
        vec![Goal::BuildMuscle, Goal::GainStrength],
    )
}

/// A high-volume aesthetic split for a tall, wide-shouldered, long-limbed frame.
/// (Arnold-70s — reconstructed: philosophy documented, granular split is
/// era-media reconstruction.)
fn classic_aesthetic_taper() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "classic-aesthetic-taper",
        "Arnold-70s",
        "Classic Aesthetic V-Taper".to_string(),
        "High-volume, high-frequency training built around the X-frame: wide \
         shoulders tapering to a small waist. A starting point to personalize from."
            .to_string(),
        FrameProfile::new(
            1.65,
            HeightBand::Tall,
            WidthBand::Wide,
            LengthBand::Long,
            Somatotype::Meso,
            vec![
                StructureTag::WideClavicles,
                StructureTag::NarrowHips,
                StructureTag::LongLimbs,
            ],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::HighVolumeSplit,
            "high-volume double split, ~6 days/week; each major muscle ~2x/week".to_string(),
            2,
            VolumeBand::High,
            "many exercises and sets per muscle, taken to failure; cheat reps and \
             antagonist supersets"
                .to_string(),
            "short, 1-2 min between sets".to_string(),
            "add volume and load progressively".to_string(),
        )?,
        DietTemplate::new(
            "old-school high-protein 'eat big' golden-era diet".to_string(),
            "generous surplus".to_string(),
            MacroEmphasis::HighProtein,
            "5-6 meals per day".to_string(),
        ),
        Provenance::new(
            Confidence::Reconstructed,
            vec![
                "Schwarzenegger & Dobbins, The New Encyclopedia of Modern Bodybuilding (1998)",
                "Arnold's Bodybuilding for Men (1981)",
                "granular day-by-day split is era-magazine reconstruction; the 'Golden Six' is contested folklore",
            ],
        ),
        vec![Goal::BuildMuscle, Goal::Recomp],
    )
}

/// A powerbuilding hybrid for a short, stocky, short-limbed frame. (Columbu —
/// reconstructed split; diet/identity documented, strength feats disputed.)
fn compact_powerbuilder() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "powerbuilder-leverage",
        "Columbu",
        "Compact Powerbuilder".to_string(),
        "Heavy compound strength fused with bodybuilding volume, suited to a \
         short, dense frame with strong leverages. A starting point to \
         personalize from."
            .to_string(),
        FrameProfile::new(
            1.45,
            HeightBand::Short,
            WidthBand::Average,
            LengthBand::Short,
            Somatotype::Meso,
            vec![StructureTag::ShortLimbs, StructureTag::DenseMuscle],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::Powerbuilding,
            "powerbuilding hybrid: heavy compounds (squat/bench/deadlift) plus \
             bodybuilding accessories; each muscle ~2x per ~14-day cycle"
                .to_string(),
            2,
            VolumeBand::Moderate,
            "heavy low-rep compound work paired with moderate-rep accessory volume".to_string(),
            "2-4 min on compounds, shorter on accessories".to_string(),
            "add load on the main lifts, reps on accessories".to_string(),
        )?,
        DietTemplate::new(
            "whole-food high-protein, egg-forward".to_string(),
            "moderate surplus".to_string(),
            MacroEmphasis::HighProtein,
            "3 meals plus 2 snacks".to_string(),
        ),
        Provenance::new(
            Confidence::Reconstructed,
            vec![
                "Franco Columbu, The Bodybuilder's Nutrition Book (1985)",
                "Franco Columbu's Complete Book of Bodybuilding",
                "14-day split is a secondary reconstruction; the famous strength feats are internally inconsistent (record as claimed, disputed)",
            ],
        ),
        vec![Goal::GainStrength, Goal::BuildMuscle],
    )
}

/// A high-volume, high-calorie mass approach for an average-height, blocky,
/// thick frame. (Cutler-00s — reconstructed: own program documented, circulating
/// templates unsourced.)
fn high_volume_mass() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "mass-monster-volume",
        "Cutler-00s",
        "High-Volume Mass Monster".to_string(),
        "A lot of food and a lot of working sets to drive maximum size on a \
         blocky, thick frame. A starting point to personalize from."
            .to_string(),
        FrameProfile::new(
            1.5,
            HeightBand::Average,
            WidthBand::Average,
            LengthBand::Average,
            Somatotype::Endo,
            vec![StructureTag::BlockyWaist, StructureTag::DenseMuscle],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::HighVolumeSplit,
            "5-day bodypart split; each muscle once weekly; back and legs split \
             into two sessions"
                .to_string(),
            1,
            VolumeBand::High,
            "~3-5 exercises per bodypart, ~20+ working sets; moderate-heavy loads \
             with strict form; pre-exhaust and drop sets"
                .to_string(),
            "30-60 s between sets".to_string(),
            "add working sets and load across the mesocycle".to_string(),
        )?,
        DietTemplate::new(
            "high-calorie 'eat to grow' clean bulk".to_string(),
            "large surplus (~4,700+ kcal)".to_string(),
            MacroEmphasis::HighCarb,
            "6-8 meals per day, eating every ~2.5-3 hours".to_string(),
        ),
        Provenance::new(
            Confidence::Reconstructed,
            vec![
                "Living Large: Jay Cutler's 8-Week Mass-Building Trainer (Bodybuilding.com)",
                "circulating split/macro templates are unsourced reconstructions; cite the published program, not the SEO articles",
            ],
        ),
        vec![Goal::BuildMuscle],
    )
}

/// A precision, high-volume hypertrophy method for a full-bellied, small-jointed,
/// tight-waisted frame. (Heath-10s — reconstructed: the method is a coach's
/// system, not athlete-authored.)
fn precision_hypertrophy() -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        "modern-precision-hypertrophy",
        "Heath-10s",
        "Precision Hypertrophy".to_string(),
        "High-volume training with strict tempo and a fascia-stretch finisher, \
         suited to a frame with full muscle bellies and small joints. A starting \
         point to personalize from."
            .to_string(),
        FrameProfile::new(
            1.55,
            HeightBand::Average,
            WidthBand::Narrow,
            LengthBand::Average,
            Somatotype::Meso,
            vec![
                StructureTag::FullMuscleBellies,
                StructureTag::SmallJoints,
                StructureTag::TightWaist,
            ],
        )?,
        ProgramTemplate::new(
            TrainingPhilosophy::ModernHypertrophy,
            "5-day bodypart split; each muscle once weekly; ~3-5 exercises per \
             bodypart"
                .to_string(),
            1,
            VolumeBand::High,
            "high volume with strict tempo and time-under-tension; a 7-set \
             fascia-stretch finisher (30-45 s rest) on the last movement"
                .to_string(),
            "45-90 s between sets; ~2-hour sessions".to_string(),
            "progress load and reps while holding strict form".to_string(),
        )?,
        DietTemplate::new(
            "individualized high-protein, auto-regulated by response".to_string(),
            "controlled surplus tuned to the individual".to_string(),
            MacroEmphasis::Balanced,
            "6-7 meals per day".to_string(),
        ),
        Provenance::new(
            Confidence::Reconstructed,
            vec![
                "FLEX magazine, June 2011, FST-7 feature (Wuebben/Rambod)",
                "the finisher method (FST-7) is Hany Rambod's system, not athlete-authored; per-athlete specifics lean on secondary profiles",
            ],
        ),
        vec![Goal::BuildMuscle, Goal::Recomp],
    )
}
