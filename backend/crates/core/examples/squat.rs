//! Run the R-0045 slice A squat model on your own numbers.
//!
//! A **developer driver**, not product code: slice C is what gives the model
//! an API (SPEC-0045 §2.9), and until it ships this is the only way to put a
//! lifter through the solve and read the answer. It adds nothing to the model
//! — every number below comes from `fitai_core::biomech`.
//!
//! ```text
//! cargo run -p fitai-core --example squat -- --height 1.80 --mass 92 --ankle 38 --bar 140
//! cargo run -p fitai-core --example squat -- --height 1.80 --mass 92 --sweep
//! cargo run -p fitai-core --example squat -- --reel
//! cargo run -p fitai-core --example squat -- --height 1.80 --mass 92 --ankle 38 --bar 140 --range
//! ```
//!
//! Measured limb lengths override the table ones: `--thigh`, `--shank`,
//! `--trunk`, in metres. Leave `--bar` off for a bodyweight squat.

use std::process::ExitCode;

use fitai_core::biomech::{
    analyse_baseline, baseline, moment_about, scale, squat_range, Baseline, Body, Dorsiflexion,
    FrameCount, Joint, Load, MeasuredLengths, Mechanics, Metres, Point, Posture, RangeEnd,
    Rejection, HIGH_BAR,
};
use fitai_core::workout::LoadKg;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("squat: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "\
usage: cargo run -p fitai-core --example squat -- [options]

  --height M      standing height, metres           (default 1.75)
  --mass KG       body mass, kilograms              (default 80)
  --ankle DEG     weight-bearing dorsiflexion       (default 40)
  --bar KG        bar load; omit for bodyweight
  --thigh M       measured hip -> knee              (default: from the table)
  --shank M       measured knee -> ankle            (default: from the table)
  --trunk M       measured hip -> shoulder          (default: from the table)
  --sweep         the dorsiflexion -> lean relation, bodyweight and loaded
  --range         the squat from standing to the bottom, frame by frame
  --frames N      frames for --range, 2..=241                 (default 11)
  --reel          MECANICA 01's published setup and its correction";

/// The default lifter: table proportions at 1.75 m, 80 kg — the fixture
/// SPEC-0045 §2.6.1.1's table is computed for.
const DEFAULT_HEIGHT_M: f64 = 1.75;
const DEFAULT_MASS_KG: f64 = 80.0;
const DEFAULT_ANKLE_DEG: f64 = 40.0;

/// Flags that take a value, and flags that stand alone. Anything else is a
/// typo, and a typo silently printing the default lifter reads as a result.
const VALUED: [&str; 8] = [
    "--height", "--mass", "--ankle", "--bar", "--thigh", "--shank", "--trunk", "--frames",
];
const SWITCHES: [&str; 5] = ["--sweep", "--range", "--reel", "--help", "-h"];

fn run(args: &[String]) -> Result<(), String> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if VALUED.contains(&arg.as_str()) {
            rest.next();
        } else if !SWITCHES.contains(&arg.as_str()) {
            return Err(format!("unknown argument {arg}"));
        }
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let height = number(args, "--height")?.unwrap_or(DEFAULT_HEIGHT_M);
    let mass_kg = number(args, "--mass")?.unwrap_or(DEFAULT_MASS_KG);
    let measured = MeasuredLengths {
        thigh: number(args, "--thigh")?.map(Metres),
        shank: number(args, "--shank")?.map(Metres),
        trunk: number(args, "--trunk")?.map(Metres),
    };
    let body = scale(Metres(height), mass_kg, &measured);
    let bar = match number(args, "--bar")? {
        Some(kg) => {
            Some(LoadKg::try_new(kg).map_err(|_| format!("--bar {kg} is not in (0, 1000]"))?)
        }
        None => None,
    };

    if args.iter().any(|a| a == "--reel") {
        return reel();
    }
    if args.iter().any(|a| a == "--sweep") {
        sweep(&body, height, mass_kg, bar, &measured);
        return Ok(());
    }

    let degrees = number(args, "--ankle")?.unwrap_or(DEFAULT_ANKLE_DEG);
    let ankle = Dorsiflexion::from_degrees(degrees)
        .ok_or_else(|| format!("--ankle {degrees} is outside 0..=90 degrees"))?;

    if args.iter().any(|a| a == "--range") {
        let requested = count(args, "--frames")?.unwrap_or(11);
        let frames = FrameCount::new(requested)
            .ok_or_else(|| format!("--frames {requested} is outside 2..=241"))?;
        range(&body, height, mass_kg, bar, ankle, frames);
        return Ok(());
    }

    println!();
    println!("Goose Physics — squat baseline (R-0045 slice A)");
    println!();
    println!(
        "  lifter      {height:.2} m, {mass_kg:.0} kg, ankle {:.1}° dorsiflexion",
        ankle.degrees()
    );
    match bar {
        Some(kg) => println!(
            "  load        {:.0} kg, high-bar at {HIGH_BAR} of the trunk line",
            kg.get()
        ),
        None => println!("  load        bodyweight only, arms hanging"),
    }
    println!();

    match baseline(&body, bar, ankle) {
        Ok(solved) => report(&body, &solved),
        Err(rejection) => explain(&body, &rejection, bar),
    }
    Ok(())
}

/// A solved lifter: the posture, the four moments, and what it all assumes.
fn report(body: &Body, solved: &Baseline) {
    let p = solved.posture();
    let lean = lean_deg(p);
    println!(
        "  posture     trunk {lean:.1}° from vertical, hip flexion {:.1}°",
        90.0 + lean
    );
    println!(
        "              shank {:.1}° · thigh parallel · segment lengths thigh {:.3} shank {:.3} trunk {:.3} m",
        shank_deg(p),
        length(p.hip, p.knee),
        length(p.knee, p.ankle),
        length(p.hip, p.shoulder),
    );
    if let Some(load) = p.load {
        println!(
            "              bar {:+.3} m from the mid-foot line",
            load.at.x.0
        );
    }
    println!();

    let mechanics: Mechanics = analyse_baseline(body, solved);
    println!("  moments     N·m resisting extension, ± the stated placement conventions");
    for moment in &mechanics.joints {
        println!(
            "    {:<10}{:>8.1}  ± {:>4.1}   {:?}",
            label(moment.joint),
            moment.moment.0,
            moment.uncertainty.0,
            moment.resisting
        );
    }
    println!();
    println!("  load basis  {:?}", mechanics.load);
    println!(
        "  assumes     {} stated assumptions:",
        mechanics.assumptions.len()
    );
    for assumption in &mechanics.assumptions {
        println!("                {assumption:?}");
    }
    println!();
}

/// A rejected lifter: why, in the words that are useful to them.
fn explain(body: &Body, rejection: &Rejection, bar: Option<LoadKg>) {
    match *rejection {
        Rejection::DepthNotReachable {
            limiting,
            needed_deg,
            available_deg,
        } => {
            println!(
                "  REJECTED    your {} is what stops you",
                label(limiting).to_lowercase()
            );
            println!(
                "              parallel needs {needed_deg:.1}° of dorsiflexion; you have {available_deg:.1}°"
            );
            if bar.is_none() {
                if let Some(under_bar) = least_ankle(body, LoadKg::try_new(100.0).ok()) {
                    println!(
                        "              with 100 kg on your back you would need only {under_bar:.1}° —"
                    );
                    println!("              the bar is a counterweight (SPEC-0045 §2.6.1.1)");
                }
            }
        }
        Rejection::DoesNotBalance { unknown } => {
            println!("  REJECTED    no trunk angle between vertical and horizontal balances");
            println!("              this body over the mid-foot line, at any dorsiflexion.");
            println!("              The unsolvable unknown is {unknown:?} — this is about");
            println!("              the proportions and the load, not about the ankle.");
        }
        Rejection::AnatomicallyImplausible { joint, angle_deg } => {
            println!("  REJECTED    {joint:?} at {angle_deg:.1}° — which should be unreachable");
            println!("              in slice A: a limit constant shipped without a citation.");
        }
    }
    println!();
}

/// SPEC-0045 §2.6.6: the squat from the solved upright toward the bottom,
/// every frame balanced, the legs moved by a stated convention.
fn range(
    body: &Body,
    height: f64,
    mass_kg: f64,
    bar: Option<LoadKg>,
    ankle: Dorsiflexion,
    frames: FrameCount,
) {
    println!();
    println!("Goose Physics — the squat over its range (R-0045 slice B1)");
    println!();
    println!(
        "  lifter      {height:.2} m, {mass_kg:.0} kg, ankle {:.1}° dorsiflexion",
        ankle.degrees()
    );
    match bar {
        Some(kg) => println!("  load        {:.0} kg, high-bar", kg.get()),
        None => println!("  load        bodyweight only, arms hanging"),
    }
    println!();
    match squat_range(body, bar, ankle, frames) {
        Err(Rejection::DepthNotReachable {
            needed_deg,
            available_deg,
            ..
        }) => {
            println!("  REJECTED    standing upright balanced already needs {needed_deg:.1}° of");
            println!(
                "              ankle in this model; you have {available_deg:.1}°, so the range"
            );
            println!("              cannot start (SPEC-0045 §2.6.6).");
        }
        Err(Rejection::DoesNotBalance { .. }) => {
            println!("  REJECTED    this body cannot stand balanced in the model: with no mass");
            println!("              on the trunk line and no bar, no lean is determined.");
        }
        Err(rejection) => explain(body, &rejection, bar),
        Ok(range) => {
            println!("      s    knee    hip  trunk |    hip   knee  ankle  L5/S1   (N·m)");
            for f in &range.frames {
                let m = |j: Joint| {
                    f.mechanics
                        .joints
                        .iter()
                        .find(|x| x.joint == j)
                        .map_or(f64::NAN, |x| x.moment.0)
                };
                println!(
                    "  {:5.3}  {:5.1}° {:5.1}° {:5.1}° | {:6.1} {:6.1} {:6.1} {:6.1}",
                    f.progress,
                    f.knee_flexion_deg,
                    f.hip_flexion_deg,
                    f.trunk_lean_deg,
                    m(Joint::Hip),
                    m(Joint::Knee),
                    m(Joint::Ankle),
                    m(Joint::Lumbar)
                );
            }
            println!();
            match range.end {
                RangeEnd::Bottom => println!("  reached the bottom (the last row is the baseline)"),
                RangeEnd::Stopped { reached, bottom } => {
                    println!("  stopped at s = {reached:.3}");
                    match bottom {
                        Some(rejection) => {
                            println!("  the bottom itself does not balance: {rejection:?}");
                        }
                        None => {
                            println!(
                                "  the bottom balances; this convention passes horizontal on the way"
                            );
                        }
                    }
                }
            }
            println!();
            println!("  Each row is a balanced posture. The legs between standing and the");
            println!("  bottom move by a stated convention, so where a moment peaks across");
            println!("  the rows is the convention's, not yours (SPEC-0045 §2.6.6). Your own");
            println!("  curve needs your own video.");
        }
    }
    println!();
}

/// SPEC-0045 §2.6.1.1's relation, for this lifter: more ankle, less lean.
fn sweep(body: &Body, height: f64, mass_kg: f64, bar: Option<LoadKg>, measured: &MeasuredLengths) {
    println!();
    println!("Goose Physics — dorsiflexion and forward lean at parallel");
    println!();
    println!("  {height:.2} m, {mass_kg:.0} kg. Trunk angle from vertical; '—' is a depth");
    println!("  this ankle cannot reach, which is a finding, not a failure.");
    println!();
    let loaded = bar.or_else(|| LoadKg::try_new(100.0).ok());
    println!("   ankle    bodyweight      loaded");
    for step in 15..=60 {
        let degrees = f64::from(step);
        let Some(ankle) = Dorsiflexion::from_degrees(degrees) else {
            continue;
        };
        if step % 5 != 0 {
            continue;
        }
        println!(
            "    {degrees:>4.0}°   {:>10}  {:>10}",
            lean_or_dash(body, None, ankle),
            lean_or_dash(body, loaded, ankle),
        );
    }
    println!();
    if let Some(bodyweight) = least_ankle(body, None) {
        println!("  reaches parallel from {bodyweight:.1}° bodyweight-only");
    }
    if let Some(under_bar) = least_ankle(body, loaded) {
        println!("                  and from {under_bar:.1}° under the bar");
    }
    if measured == &MeasuredLengths::default() {
        println!();
        println!("  The bodyweight column is the same for any height and mass at table");
        println!("  proportions — it depends on the ratios, and they do not vary. Pass");
        println!("  --thigh/--shank/--trunk to see your own. (The loaded column does");
        println!("  move: the bar is an absolute mass against a scaled body.)");
    }
    println!();
}

/// MECÁNICA 01's published setup, and what the model says about it
/// (SPEC-0045 §2.4.3). The reel's own lengths, as measured overrides.
fn reel() -> Result<(), String> {
    let measured = MeasuredLengths {
        thigh: Some(Metres(0.40)),
        shank: Some(Metres(0.43)),
        trunk: Some(Metres(0.53)),
    };
    let lifter = scale(Metres(1.75), 80.0, &measured);
    let bar = LoadKg::try_new(100.0).map_err(|_| "100 kg is a valid load".to_owned())?;

    println!();
    println!("Goose Physics — MECÁNICA 01, checked");
    println!();
    println!("  setup       80 kg lifter, 100 kg bar, femur 0.40 m, shank 0.43 m,");
    println!("              trunk 0.53 m, shank 22° from vertical");
    println!();

    let posture = reel_posture(bar);
    let massless = scale(Metres(1.75), 0.0, &measured);
    let total = moment_about(Joint::Hip, &lifter, &posture).0;
    let component = moment_about(Joint::Hip, &massless, &posture).0;

    println!("  published   274 N·m at the hip");
    println!("  bar only    {component:.1} N·m  — the reel's number, recovered as a component");
    println!("  honest      {total:.1} N·m  — the bar plus the lifter's own segments");
    println!(
        "              the true figure is {:.0}% higher than the published one",
        100.0 * (total - component) / component
    );
    println!();
    for joint in [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar] {
        println!(
            "    {:<10}{:>8.1} N·m",
            label(joint),
            moment_about(joint, &lifter, &posture).0
        );
    }
    println!();
    println!("  This posture is **built**, not solved: it is the reel's own geometry,");
    println!("  with the trunk set so the bar sits over the mid-foot. So there is no");
    println!("  `Mechanics` here — `analyse_baseline` takes a `Baseline`, which only");
    println!("  the solver can make, and it would be a lie to stamp `SolvedBaseline`");
    println!("  on someone else's drawing (SPEC-0045 §2.1).");
    println!();
    println!("  The reel is a bar-only model and says so nowhere. Whether to publish");
    println!("  the correction is the owner's call (SPEC-0045 §5.4).");
    println!();
    Ok(())
}

/// MECÁNICA 01's posture, laid out by hand from the reel's own figures: its
/// ankle, its 22° shank, and the trunk angle that puts the bar over the
/// mid-foot line. The same construction `B3` uses.
fn reel_posture(bar: LoadKg) -> Posture {
    let point = |x: f64, y: f64| Point {
        x: Metres(x),
        y: Metres(y),
    };
    let (shank, femur, trunk) = (0.43, 0.40, 0.53);
    let ankle = point(-0.04, 0.09);
    let (sin_shank, cos_shank) = 22.0_f64.to_radians().sin_cos();
    let knee = point(ankle.x.0 + shank * sin_shank, ankle.y.0 + shank * cos_shank);
    let hip = point(knee.x.0 - femur, knee.y.0);
    let (sin_trunk, cos_trunk) = (-hip.x.0 / (HIGH_BAR * trunk)).asin().sin_cos();
    let shoulder = point(hip.x.0 + trunk * sin_trunk, hip.y.0 + trunk * cos_trunk);
    let at = point(
        hip.x.0 + HIGH_BAR * (shoulder.x.0 - hip.x.0),
        hip.y.0 + HIGH_BAR * (shoulder.y.0 - hip.y.0),
    );
    Posture {
        ankle,
        knee,
        hip,
        shoulder,
        load: Some(Load { kg: bar, at }),
    }
}

/// The least dorsiflexion that reaches parallel, by asking the model: the
/// rejection carries it, so no threshold is recomputed here.
fn least_ankle(body: &Body, bar: Option<LoadKg>) -> Option<f64> {
    let floor = Dorsiflexion::from_degrees(0.0)?;
    match baseline(body, bar, floor) {
        Err(Rejection::DepthNotReachable { needed_deg, .. }) => Some(needed_deg),
        _ => None,
    }
}

fn lean_or_dash(body: &Body, bar: Option<LoadKg>, ankle: Dorsiflexion) -> String {
    baseline(body, bar, ankle).map_or_else(
        |_| "—".to_owned(),
        |solved| format!("{:.1}°", lean_deg(solved.posture())),
    )
}

fn lean_deg(p: &Posture) -> f64 {
    (p.shoulder.x.0 - p.hip.x.0)
        .atan2(p.shoulder.y.0 - p.hip.y.0)
        .to_degrees()
}

fn shank_deg(p: &Posture) -> f64 {
    (p.knee.x.0 - p.ankle.x.0)
        .atan2(p.knee.y.0 - p.ankle.y.0)
        .to_degrees()
}

fn length(a: fitai_core::biomech::Point, b: fitai_core::biomech::Point) -> f64 {
    (b.x.0 - a.x.0).hypot(b.y.0 - a.y.0)
}

fn label(joint: Joint) -> &'static str {
    match joint {
        Joint::Hip => "hip",
        Joint::Knee => "knee",
        Joint::Ankle => "ankle",
        Joint::Lumbar => "L5/S1",
    }
}

/// `--flag VALUE` as a whole count, or `None` if the flag is absent.
fn count(args: &[String], name: &str) -> Result<Option<u16>, String> {
    let Some(index) = args.iter().position(|a| a == name) else {
        return Ok(None);
    };
    let raw = args
        .get(index + 1)
        .ok_or_else(|| format!("{name} needs a value"))?;
    raw.parse::<u16>()
        .map(Some)
        .map_err(|_| format!("{name} {raw} is not a whole number"))
}

/// `--flag VALUE` as a number, or `None` if the flag is absent.
fn number(args: &[String], name: &str) -> Result<Option<f64>, String> {
    let Some(index) = args.iter().position(|a| a == name) else {
        return Ok(None);
    };
    let raw = args
        .get(index + 1)
        .ok_or_else(|| format!("{name} needs a value"))?;
    raw.parse::<f64>()
        .map(Some)
        .map_err(|_| format!("{name} {raw} is not a number"))
}
