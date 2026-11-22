//! SPEC-0066 §2.1.2 — the mechanics kernel glue: the **only** module that
//! imports garust (ADR 0001).
//!
//! A [`PlanarTree`] is built **at a posture**: each link's frame sits at its
//! joint's centre, every joint is a revolute about `z` (out of the sagittal
//! plane) through that centre, and the tree is evaluated in that reference
//! configuration (`q = 0`). So the geometry is the posture's own points, and no
//! angle bookkeeping stands between a posture and its moments.
//!
//! Every moment is computed twice — `joint_torque` (the moment of everything
//! distal, a cross product) and `generalized_torque` (virtual work, a
//! gradient) — and served only when the two agree (R-0066 AC5). A cross
//! product and a gradient do not make the same mistake.
//!
//! The world origin is the tree's root, so `|F·x|` stays small and the
//! finite-difference cross-check's rounding stays bounded (SPEC-0066 §2.1.2).

use garust_core::Pga3;
use garust_geo::chain::ChainJoint;
use garust_geo::tree::{Tree, TreeLink};
use garust_geo::Motor;
use garust_physics::load::{Load, Weight};
use garust_physics::multibody::{generalized_torque, joint_torque};

use super::{Joint, NewtonMetres, Point, G};

/// The virtual-work step, radians — the parity spike's (SPEC-0066 §2.1.2).
pub(crate) const FD_STEP: f64 = 1e-6;

/// The cross-check's absolute tolerance, N·m.
const TOL_ABS: f64 = 1e-4;

/// The cross-check's relative tolerance: rounding in the finite difference
/// grows with the load, so heavy loads get a proportional margin.
const TOL_REL: f64 = 1e-9;

/// Which anatomical segment is on the tree's distal side of a joint
/// (SPEC-0066 §2.1.3). It decides how garust's `τ_z` maps to "resisting".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The tree's distal side is the joint's anatomical distal segment
    /// (rooted at the shoulder, the elbow's distal side is the forearm).
    Anatomical,
    /// The tree's distal side is the joint's anatomical proximal segment
    /// (rooted at the foot, the knee's distal side is the thigh and above).
    Proximal,
}

impl Side {
    fn sign(self) -> f64 {
        match self {
            Self::Anatomical => 1.0,
            Self::Proximal => -1.0,
        }
    }
}

/// A link of a [`PlanarTree`]. Only [`PlanarTree::link`] makes one, so within
/// one tree a parent always precedes its child and garust's ordering rule
/// cannot fail. An id is only meaningful in the tree that made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinkId(usize);

/// The kernel and its cross-check disagreed: a modelling bug, never served
/// (R-0066 AC7). The API maps it to `500` and logs no body value.
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum KernelError {
    /// `joint_torque` and `generalized_torque` differ beyond tolerance.
    #[error("the {joint:?} moment disagrees: cross product {cross} N·m, virtual work {virtual_work} N·m")]
    Disagree {
        /// The joint whose two derivations differ.
        joint: Joint,
        /// The cross-product moment, garust's `τ_z`.
        cross: f64,
        /// The virtual-work moment.
        virtual_work: f64,
    },
    /// A model asked for a joint its tree does not report: a bug in the
    /// model's tree, never a silent zero.
    #[error("the {joint:?} moment was asked for but the tree does not report it")]
    NotReported {
        /// The joint asked for.
        joint: Joint,
    },
}

impl KernelError {
    /// The joint the error is about, for a log line that carries no body value.
    #[must_use]
    pub fn joint(self) -> Joint {
        match self {
            Self::Disagree { joint, .. } | Self::NotReported { joint } => joint,
        }
    }
}

/// A planar kinematic tree for garust, built at a posture: links parent
/// first, the joints reported with their [`Side`], and the weights on it.
#[derive(Clone, Debug, Default)]
pub(crate) struct PlanarTree {
    links: Vec<TreeLink>,
    /// Each link's joint centre, world frame (the posture's coordinates).
    centres: Vec<Point>,
    reported: Vec<(LinkId, Joint, Side)>,
    weights: Vec<Weight>,
}

impl PlanarTree {
    /// An empty tree.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Adds a link whose joint, a revolute about `z`, sits at `centre`
    /// (posture coordinates), hanging from `parent` (`None` for the root).
    ///
    /// The axis passes through the link's frame origin by construction
    /// (SPEC-0066 §2.1.2's invariant): the frame origin *is* `centre`.
    ///
    /// # Panics
    ///
    /// If `parent` is a [`LinkId`] from another, smaller tree: a programming
    /// error, not a runtime condition (every caller builds its tree in one
    /// function from ids that tree returned).
    pub(crate) fn link(&mut self, parent: Option<LinkId>, centre: Point) -> LinkId {
        let from = parent.map_or(self.root_centre_or(centre), |p| self.centres[p.0]);
        self.links.push(TreeLink {
            parent: parent.map(|p| p.0),
            offset: Motor::translator(centre.x.0 - from.x.0, centre.y.0 - from.y.0, 0.0),
            joint: ChainJoint::Revolute(z_axis()),
        });
        self.centres.push(centre);
        LinkId(self.links.len() - 1)
    }

    /// Reports `link`'s joint as `joint`, with the tree's distal side `side`.
    /// A [`LinkId`] from another tree makes [`moments`] panic, as in
    /// [`PlanarTree::link`].
    pub(crate) fn report(&mut self, link: LinkId, joint: Joint, side: Side) {
        self.reported.push((link, joint, side));
    }

    /// Hangs `kg` at `at` (posture coordinates) on `link`, under gravity.
    ///
    /// # Panics
    ///
    /// If `link` is a [`LinkId`] from another, smaller tree, as in
    /// [`PlanarTree::link`].
    pub(crate) fn weight(&mut self, link: LinkId, at: Point, kg: f64) {
        let centre = self.centres[link.0];
        self.weights.push(Weight {
            link: link.0,
            offset: [at.x.0 - centre.x.0, at.y.0 - centre.y.0, 0.0],
            mass: kg,
            gravity: [0.0, -G, 0.0],
        });
    }

    /// Every reported joint's centre as garust's forward kinematics places
    /// it, back in posture coordinates. The FK invariant test compares these
    /// with the posture (SPEC-0066 §2.1.2).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn joint_centres(&self) -> Vec<(Joint, Point)> {
        use garust_physics::multibody::joint_point;

        if self.links.is_empty() {
            return Vec::new();
        }
        let poses = reference_poses(&Tree::new(&self.links));
        let root = self.centres[0];
        self.reported
            .iter()
            .map(|&(link, joint, _)| {
                let [x, y, _] = joint_point(&poses, link.0);
                (joint, point(root.x.0 + x, root.y.0 + y))
            })
            .collect()
    }

    /// The root's centre, the world origin; `fallback` while the tree is empty.
    fn root_centre_or(&self, fallback: Point) -> Point {
        self.centres.first().copied().unwrap_or(fallback)
    }
}

/// Forward kinematics at the reference configuration (`q = 0`).
fn reference_poses(tree: &Tree<'_>) -> Vec<Motor<f64>> {
    let mut poses = vec![Motor::identity(); tree.dof()];
    tree.fk(&vec![0.0; tree.dof()], &mut poses);
    poses
}

/// The signed moment (SPEC-0066 §2.1.3: positive when the loads tend to flex
/// the joint) at every reported joint, in report order, each cross-checked by
/// virtual work.
///
/// # Errors
///
/// [`KernelError::Disagree`] when a joint's two derivations differ by more
/// than `1e-4 + 1e-9·|τ|` N·m.
///
/// # Panics
///
/// If a reported [`LinkId`] came from another, larger tree (see
/// [`PlanarTree::link`]).
pub(crate) fn moments(tree: &PlanarTree) -> Result<Vec<(Joint, NewtonMetres)>, KernelError> {
    if tree.links.is_empty() {
        return Ok(Vec::new());
    }
    let garust_tree = Tree::new(&tree.links);
    let poses = reference_poses(&garust_tree);
    let loads: Vec<&dyn Load> = tree.weights.iter().map(|w| w as &dyn Load).collect();
    let q = vec![0.0; garust_tree.dof()];
    let mut work = poses.clone();
    let mut scratch = vec![0.0; garust_tree.dof()];
    tree.reported
        .iter()
        .map(|&(link, joint, side)| {
            let cross = joint_torque(&garust_tree, &poses, &loads, link.0)[2];
            let virtual_work = generalized_torque(
                &garust_tree,
                &loads,
                &q,
                link.0,
                FD_STEP,
                &mut work,
                &mut scratch,
            );
            if !agree(cross, virtual_work) {
                return Err(KernelError::Disagree {
                    joint,
                    cross,
                    virtual_work,
                });
            }
            Ok((
                joint,
                NewtonMetres(flexion_dir(joint) * side.sign() * cross),
            ))
        })
        .collect()
}

/// `joint`'s moment in `moments`' output.
///
/// # Errors
///
/// [`KernelError::NotReported`] when the tree did not report `joint`.
pub(crate) fn moment_of(
    moments: &[(Joint, NewtonMetres)],
    joint: Joint,
) -> Result<NewtonMetres, KernelError> {
    moments
        .iter()
        .find(|(j, _)| *j == joint)
        .map(|&(_, m)| m)
        .ok_or(KernelError::NotReported { joint })
}

/// Whether two derivations of one moment agree: `|a − b| ≤ 1e-4 + 1e-9·|a|`.
/// Symmetric only approximately: the relative term scales with the cross
/// product's magnitude, the reference.
#[must_use]
pub(crate) fn agree(cross: f64, virtual_work: f64) -> bool {
    (cross - virtual_work).abs() <= TOL_ABS + TOL_REL * cross.abs()
}

/// The rotation sense, in the right-hand sense of `joint_torque`'s `τ_z`,
/// that flexes the joint when its anatomical distal segment moves, the lifter
/// facing `+x` (SPEC-0066 §2.1.3). For L5/S1 the distal segment is the thorax.
fn flexion_dir(joint: Joint) -> f64 {
    match joint {
        // The thigh swings forward and up; the foot's toes rise: counter-clockwise.
        Joint::Hip | Joint::Ankle => 1.0,
        // The shank swings back; the thorax tips forward: clockwise.
        Joint::Knee | Joint::Lumbar => -1.0,
    }
}

/// The line through the frame origin along `z`, out of the sagittal plane.
fn z_axis() -> Pga3 {
    Pga3::point(0.0, 0.0, 0.0).line_through(&Pga3::point(0.0, 0.0, 1.0))
}

#[cfg(test)]
fn point(x: f64, y: f64) -> Point {
    Point {
        x: super::Metres(x),
        y: super::Metres(y),
    }
}
