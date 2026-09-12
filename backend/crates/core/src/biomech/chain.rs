//! SPEC-0045 §2.4 — the static chain: what sits on the load side of each
//! joint, where its mass acts, and the signed moment about the joint.
//!
//! Quasi-static by decision (OQ-2): every joint in moment equilibrium, no
//! accelerations — exact at the sticking point, where the lifter is slowest
//! and the coaching question lives, and an understatement during fast phases.
//! This module is the **only** place gravity is multiplied and the **only**
//! place a sign is applied (SPEC-0045 §3).

use super::{Body, Joint, NewtonMetres, Point, Posture};

/// A lumped mass on the load side of a joint (SPEC-0045 §2.4.2). `Shank` and
/// `Thigh` are both legs and `Arms` both arms — the sagittal model
/// superimposes the pairs. The external load is not a segment; it travels on
/// [`Posture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChainSegment {
    /// Knee → ankle, both legs.
    Shank,
    /// Hip → knee, both legs.
    Thigh,
    /// L4-L5 → greater trochanter; on the hip's load side in full, a stated
    /// convention with its error term.
    Pelvis,
    /// C7-T1 → L4-L5.
    ThoraxAbdomen,
    /// Placed by convention on the trunk line beyond the shoulder.
    HeadNeck,
    /// Both arms: on the bar when there is one, hanging when there is not.
    Arms,
}

impl ChainSegment {
    /// Where this segment's centre of mass sits in `p`, by the §2.3 COM
    /// fractions and the §2.4.2 placement conventions: the head & neck on the
    /// trunk line beyond the shoulder; the arms at the shoulder → load
    /// midpoint when there is a load, and hanging directly below the shoulder
    /// when there is not.
    #[must_use]
    pub fn com(self, _body: &Body, _p: &Posture) -> Point {
        todo!("R-0045 slice A")
    }
}

/// The load side of `j`: four fixed lists, a serial chain summed from the top,
/// not a graph walk (SPEC-0045 §2.4.2).
#[must_use]
pub fn above(_j: Joint) -> &'static [ChainSegment] {
    todo!("R-0045 slice A")
}

/// The moment about `j` in N·m **resisting extension**: positive means the
/// extensors are loaded; negative means the load is assisting extension,
/// which is a real state (a bar behind the hip). Signed, never `.abs()`
/// (SPEC-0045 §2.4.1).
#[must_use]
pub fn moment_about(_j: Joint, _body: &Body, _p: &Posture) -> NewtonMetres {
    todo!("R-0045 slice A")
}
