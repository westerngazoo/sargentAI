//! SPEC-0045 §2.3 — anthropometrics: Winter's table, scaled to a person.
//!
//! Source: Winter, D. A., *Biomechanics and Motor Control of Human Movement*,
//! 4th ed., Wiley 2009 — Table 4.1 (segment masses and centres of mass, after
//! Dempster 1955) and Figure 4.1 (segment lengths and breadths as fractions of
//! height, after Drillis & Contini 1966). No invented constants (R-0045 AC2).
//!
//! The AC2 control is two-fold (SPEC-0045 §2.3): [`WINTER_VERIFICATION`]
//! records who checked the values against which edition, and the table is
//! self-checking through three sum claims that must hold whatever the values
//! are —
//!
//! ```text
//! HAT + 2·(thigh + shank + foot)                          = 1.000
//! trunk + head&neck + 2·(upper arm + forearm + hand)      = HAT
//! thorax&abdomen + pelvis                                 = trunk
//! ```
//!
//! **Body fat** (OQ-1): the table does not vary by adiposity and this module
//! does not invent an adjustment. **Individual variation:** these are
//! population means with real spread. Every *length* is overridable by a
//! measurement through [`MeasuredLengths`]; every *mass* is not — masses have
//! no measurement path and are always table-derived.

use super::{ChainSegment, Metres};

/// One row of Table 4.1 / Figure 4.1. Never constructed outside [`WINTER`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// Segment mass as a fraction of body mass `M` (Table 4.1).
    pub mass_fraction: f64,
    /// Centre of mass as a fraction of segment length from the **proximal**
    /// end (Table 4.1).
    pub com_from_proximal: f64,
    /// Segment length as a fraction of height `H` (Figure 4.1); `None` where
    /// the figure gives no length for the row.
    pub length_fraction_of_height: Option<f64>,
    /// Where the row comes from: the table or figure and the row's name there,
    /// or the SPEC-0045 section that states a convention.
    pub citation: &'static str,
}

/// A body dimension with no mass of its own (Figure 4.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Length {
    /// As a fraction of height `H`.
    pub fraction_of_height: f64,
    /// Where the value comes from.
    pub citation: &'static str,
}

/// The §2.3 table. Each field's doc gives the row's proximal → distal
/// definition; the value's `citation` gives its source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Table {
    /// Ankle → toe.
    pub foot: Segment,
    /// Knee → ankle.
    pub shank: Segment,
    /// Hip → knee.
    pub thigh: Segment,
    /// L4-L5 → greater trochanter.
    pub pelvis: Segment,
    /// C7-T1 → L4-L5.
    pub thorax_abdomen: Segment,
    /// Shoulder → hip; `pelvis + thorax_abdomen` by mass — the split the
    /// lumbar row needs (SPEC-0045 §2.5).
    pub trunk: Segment,
    /// C7-T1 → vertex.
    pub head_neck: Segment,
    /// Shoulder → elbow.
    pub upper_arm: Segment,
    /// Elbow → wrist.
    pub forearm: Segment,
    /// Wrist → knuckle.
    pub hand: Segment,
    /// Head, arms and trunk, greater trochanter → glenohumeral: the lump whose
    /// COM assumes arms at the sides — which is why the model uses the split
    /// (SPEC-0045 §2.4.2). Kept for the sum claim and the reel correction.
    pub hat: Segment,
    /// Hip breadth — the stance-width knob's unit (slice B).
    pub hip_breadth: Length,
    /// Shoulder breadth.
    pub shoulder_breadth: Length,
    /// Floor → ankle joint — a baseline solve input (§2.6.1).
    pub ankle_height: Length,
}

/// Winter (2009) Table 4.1 and Figure 4.1, exactly as SPEC-0045 §2.3 lists
/// them. Mass as a fraction of body mass `M`; centre of mass as a fraction of
/// segment length from the proximal end; length as a fraction of height `H`.
///
/// The one value that is not Winter's is the head & neck COM: Table 4.1 gives
/// the lumped row no usable centre, and SPEC-0045 §2.4.2 places it by
/// convention half a head & neck length beyond the shoulder — carried here so
/// the convention has one home.
pub const WINTER: Table = Table {
    foot: Segment {
        mass_fraction: 0.0145,
        com_from_proximal: 0.50,
        length_fraction_of_height: Some(0.152),
        citation: "Winter 2009, Table 4.1 'Foot'; Figure 4.1, foot length 0.152 H",
    },
    shank: Segment {
        mass_fraction: 0.0465,
        com_from_proximal: 0.433,
        length_fraction_of_height: Some(0.246),
        citation: "Winter 2009, Table 4.1 'Leg'; Figure 4.1, shank length 0.246 H",
    },
    thigh: Segment {
        mass_fraction: 0.100,
        com_from_proximal: 0.433,
        length_fraction_of_height: Some(0.245),
        citation: "Winter 2009, Table 4.1 'Thigh'; Figure 4.1, thigh length 0.245 H",
    },
    pelvis: Segment {
        mass_fraction: 0.142,
        com_from_proximal: 0.105,
        length_fraction_of_height: None,
        citation: "Winter 2009, Table 4.1 'Pelvis' (L4-L5 to greater trochanter)",
    },
    thorax_abdomen: Segment {
        mass_fraction: 0.355,
        com_from_proximal: 0.63,
        length_fraction_of_height: None,
        citation: "Winter 2009, Table 4.1 'Thorax and abdomen' (C7-T1 to L4-L5)",
    },
    trunk: Segment {
        mass_fraction: 0.497,
        com_from_proximal: 0.50,
        length_fraction_of_height: Some(0.288),
        citation: "Winter 2009, Table 4.1 'Trunk'; Figure 4.1, trunk length 0.288 H",
    },
    head_neck: Segment {
        mass_fraction: 0.081,
        com_from_proximal: 0.5,
        length_fraction_of_height: Some(0.130),
        citation: "Winter 2009, Table 4.1 'Head and neck' (mass); Figure 4.1, \
                   vertex to chin 0.130 H; COM 0.5 is the SPEC-0045 §2.4.2 \
                   placement convention, not a Table 4.1 value",
    },
    upper_arm: Segment {
        mass_fraction: 0.028,
        com_from_proximal: 0.436,
        length_fraction_of_height: Some(0.186),
        citation: "Winter 2009, Table 4.1 'Upper arm'; Figure 4.1, upper arm 0.186 H",
    },
    forearm: Segment {
        mass_fraction: 0.016,
        com_from_proximal: 0.430,
        length_fraction_of_height: Some(0.146),
        citation: "Winter 2009, Table 4.1 'Forearm'; Figure 4.1, forearm 0.146 H",
    },
    hand: Segment {
        mass_fraction: 0.006,
        com_from_proximal: 0.506,
        length_fraction_of_height: Some(0.108),
        citation: "Winter 2009, Table 4.1 'Hand'; Figure 4.1, hand 0.108 H",
    },
    hat: Segment {
        mass_fraction: 0.678,
        com_from_proximal: 0.626,
        length_fraction_of_height: None,
        citation: "Winter 2009, Table 4.1 'HAT' (greater trochanter to glenohumeral joint)",
    },
    hip_breadth: Length {
        fraction_of_height: 0.191,
        citation: "Winter 2009, Figure 4.1, hip breadth 0.191 H",
    },
    shoulder_breadth: Length {
        fraction_of_height: 0.259,
        citation: "Winter 2009, Figure 4.1, shoulder breadth 0.259 H",
    },
    ankle_height: Length {
        fraction_of_height: 0.039,
        citation: "Winter 2009, Figure 4.1, ankle height 0.039 H",
    },
};

/// The AC2 verification record (SPEC-0045 §2.3, control (i)): the edition,
/// pages, date and person that checked [`WINTER`] against the printed source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verification {
    /// The values are the canonical ones and pass the sum claims, but a
    /// recollection is not the control.
    Pending,
    /// Checked against the printed source.
    Verified {
        /// Edition consulted.
        edition: &'static str,
        /// Pages of the table and figure.
        pages: &'static str,
        /// When, ISO date.
        date: &'static str,
        /// Who.
        by: &'static str,
    },
}

/// *Pending* — as SPEC-0045 §2.3 has it until someone opens the book.
pub const WINTER_VERIFICATION: Verification = Verification::Pending;

/// Lengths a user has had measured, in metres. `None` means table-derived
/// from height (SPEC-0045 §2.1.1, §2.3). Height scales only the unmeasured
/// segments (architect finding 16).
///
/// Ankle dorsiflexion is *not* here: it is an angle, not a length, and unlike
/// a length it has no table fallback to be measured *instead of*
/// (SPEC-0045 §2.6.1.1). It is a required argument to the solve.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeasuredLengths {
    /// Hip → knee (femur).
    pub thigh: Option<Metres>,
    /// Knee → ankle.
    pub shank: Option<Metres>,
    /// Hip → shoulder.
    pub trunk: Option<Metres>,
}

/// One person: the table scaled by height and mass, with any measured length
/// overriding its table value. Built by [`scale`] only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    mass_kg: f64,
    thigh: Metres,
    shank: Metres,
    trunk: Metres,
    foot: Metres,
    head_neck: Metres,
    upper_arm: Metres,
    forearm: Metres,
    hand: Metres,
    ankle_height: Metres,
    any_length_table_derived: bool,
}

impl Body {
    /// Mass in kilograms of one lumped chain segment — both legs for `Shank`
    /// and `Thigh`, both arms for `Arms` (the sagittal model superimposes the
    /// pairs). Always table-derived: masses have no measurement path.
    #[must_use]
    pub fn mass_kg(&self, segment: ChainSegment) -> f64 {
        let fraction = match segment {
            ChainSegment::Shank => 2.0 * WINTER.shank.mass_fraction,
            ChainSegment::Thigh => 2.0 * WINTER.thigh.mass_fraction,
            ChainSegment::Pelvis => WINTER.pelvis.mass_fraction,
            ChainSegment::ThoraxAbdomen => WINTER.thorax_abdomen.mass_fraction,
            ChainSegment::HeadNeck => WINTER.head_neck.mass_fraction,
            ChainSegment::Arms => {
                2.0 * (WINTER.upper_arm.mass_fraction
                    + WINTER.forearm.mass_fraction
                    + WINTER.hand.mass_fraction)
            }
        };
        fraction * self.mass_kg
    }

    /// Hip → knee.
    pub(super) fn thigh(&self) -> f64 {
        self.thigh.0
    }

    /// Knee → ankle.
    pub(super) fn shank(&self) -> f64 {
        self.shank.0
    }

    /// Hip → shoulder.
    pub(super) fn trunk(&self) -> f64 {
        self.trunk.0
    }

    /// Heel → toe.
    pub(super) fn foot(&self) -> f64 {
        self.foot.0
    }

    /// C7-T1 → vertex; the head & neck COM is placed by half of it
    /// (SPEC-0045 §2.4.2).
    pub(super) fn head_neck(&self) -> f64 {
        self.head_neck.0
    }

    /// Shoulder → elbow.
    pub(super) fn upper_arm(&self) -> f64 {
        self.upper_arm.0
    }

    /// Elbow → wrist.
    pub(super) fn forearm(&self) -> f64 {
        self.forearm.0
    }

    /// Wrist → knuckle.
    pub(super) fn hand(&self) -> f64 {
        self.hand.0
    }

    /// Floor → ankle joint (SPEC-0045 §2.6.1).
    pub(super) fn ankle_height(&self) -> f64 {
        self.ankle_height.0
    }

    /// Whether any length came from the table rather than a measurement — the
    /// condition `Assumption::TableDerivedLengths` is reported under.
    pub(super) fn any_length_table_derived(&self) -> bool {
        self.any_length_table_derived
    }
}

/// A row's length at `height`. Total: a row Figure 4.1 gives no length gets
/// zero rather than a panic in library code (constitution §6). No row scaled
/// by [`scale`] is one of those, which is what
/// `every_scaled_row_has_a_table_length` exists to keep true.
fn table_length(row: &Segment, height: Metres) -> Metres {
    Metres(row.length_fraction_of_height.unwrap_or(0.0) * height.0)
}

/// The lengths the model needs that [`MeasuredLengths`] has no field for,
/// and so can never override — the arm's three segments counted as one. They are why `TableDerivedLengths` is on
/// every v1 result whatever the lifter has had measured — named rather than
/// folded into a `true`, so that the day one of them becomes measurable the
/// condition changes with it (SPEC-0045 §2.8).
const FOOT_HAS_NO_MEASUREMENT_PATH: bool = true;
const HEAD_NECK_HAS_NO_MEASUREMENT_PATH: bool = true;
const ARM_HAS_NO_MEASUREMENT_PATH: bool = true;

/// Scale the table to a person. `measured` overrides any **length** the user
/// has had measured; masses are always table-derived (SPEC-0045 §2.3).
///
/// Height scales only the *unmeasured* segments (architect finding 16): a
/// lifter with a measured femur and a table-derived shank gets exactly that,
/// not a femur quietly re-derived from their height.
#[must_use]
pub fn scale(height: Metres, mass_kg: f64, measured: &MeasuredLengths) -> Body {
    Body {
        mass_kg,
        thigh: measured
            .thigh
            .unwrap_or_else(|| table_length(&WINTER.thigh, height)),
        shank: measured
            .shank
            .unwrap_or_else(|| table_length(&WINTER.shank, height)),
        trunk: measured
            .trunk
            .unwrap_or_else(|| table_length(&WINTER.trunk, height)),
        foot: table_length(&WINTER.foot, height),
        head_neck: table_length(&WINTER.head_neck, height),
        upper_arm: table_length(&WINTER.upper_arm, height),
        forearm: table_length(&WINTER.forearm, height),
        hand: table_length(&WINTER.hand, height),
        ankle_height: Metres(WINTER.ankle_height.fraction_of_height * height.0),
        any_length_table_derived: [
            measured.thigh.is_none(),
            measured.shank.is_none(),
            measured.trunk.is_none(),
            FOOT_HAS_NO_MEASUREMENT_PATH,
            HEAD_NECK_HAS_NO_MEASUREMENT_PATH,
            ARM_HAS_NO_MEASUREMENT_PATH,
        ]
        .iter()
        .any(|&from_the_table| from_the_table),
    }
}
