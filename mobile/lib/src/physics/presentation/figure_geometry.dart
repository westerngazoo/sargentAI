// SPEC-0059 §2.4 — where the squat figure's parts land on the canvas. Pure
// geometry, so it is tested without a widget. Display only (R-0059 AC7): the
// lengths of the force vectors are the returned frame's newtons on one
// scale; nothing here computes a force or a moment.

import 'dart:math' as math;
import 'dart:ui';

import '../domain/moment_scale.dart';
import '../domain/playback.dart';
import '../domain/squat_range.dart';

/// A barbell plate's radius, metres: the 450 mm plate, so the bar's plate
/// is drawn to scale.
const plateRadiusM = 0.225;

/// The largest ground force spans this fraction of the canvas height; every
/// other vector is drawn to the same newtons per pixel.
const maxVectorFraction = 0.4;

// Drawing conventions in metres, not part of the model: the head sits on
// the trunk's line, the foot runs from just behind the ankle to the toes,
// and a hanging arm is about as long as the trunk. With a bar the model
// places the hands on it (SPEC-0059 §2.4 caveats), the elbow below and
// behind it.
const _headGapM = 0.15;
const _headRadiusM = 0.1;
const _heelM = 0.06;
const _toesM = 0.2;
const _armPerTrunk = 1.1;
const _elbowFromBar = Point(-0.03, -0.19);
const _padPx = 8.0;

/// A line segment on the canvas.
typedef Segment = (Offset, Offset);

/// Metres to canvas for a whole range: `x` forward stays right, `y` up
/// becomes canvas up.
class FigureFit {
  const FigureFit(this.scale, this.origin);

  /// Logical pixels per metre.
  final double scale;

  /// Where the mid-foot on the ground, `(0, 0)` m, lands.
  final Offset origin;

  Offset toCanvas(Point p) =>
      Offset(origin.dx + p.x * scale, origin.dy - p.y * scale);
}

/// A force drawn as an arrow from [from] to [to]. [heat], `0..=1`, is the
/// tension it loads for the ramp (SPEC-0061 §2.10: heat is tension magnitude
/// only); null draws it in ink, as the ground's reaction is.
class ForceVector {
  const ForceVector(this.from, this.to, this.heat);

  final Offset from;
  final Offset to;
  final double? heat;

  double get length => (to - from).distance;
}

/// Everything the painter draws for one playback frame, in canvas pixels.
class FigureGeometry {
  const FigureGeometry({
    required this.scale,
    required this.groundY,
    required this.ankle,
    required this.knee,
    required this.hip,
    required this.shoulder,
    required this.head,
    required this.headRadius,
    required this.foot,
    required this.arm,
    required this.bar,
    required this.plateRadius,
    required this.balanceLine,
    required this.lever,
    required this.ground,
    required this.barForce,
  });

  /// Logical pixels per metre, for the painter's stroke widths.
  final double scale;
  final double groundY;
  final Offset ankle;
  final Offset knee;
  final Offset hip;
  final Offset shoulder;
  final Offset head;
  final double headRadius;

  /// Heel to toes, on the ground.
  final Segment foot;

  /// Shoulder, (elbow,) hand: to the bar with one, hanging without.
  final List<Offset> arm;

  /// The bar's centre; null for bodyweight only.
  final Offset? bar;
  final double plateRadius;

  /// The mid-foot vertical, from the ground up past the head.
  final Segment balanceLine;

  /// From the hip, level, to the bar's vertical line; null without a bar.
  final Segment? lever;

  /// The ground's reaction, up from the mid-foot, in ink (no heat).
  final ForceVector ground;

  /// The bar's weight, down from under the plate, heated by the frame's hip
  /// moment against the range's [maxAbsMoment]; null for bodyweight only.
  final ForceVector? barForce;
}

Point _add(Point a, Point b) => Point(a.x + b.x, a.y + b.y);

Point _head(Posture p) {
  final dx = p.shoulder.x - p.hip.x, dy = p.shoulder.y - p.hip.y;
  final k = _headGapM / math.sqrt(dx * dx + dy * dy);
  return Point(p.shoulder.x + dx * k, p.shoulder.y + dy * k);
}

Point _hangingHand(Posture p) {
  final dx = p.shoulder.x - p.hip.x, dy = p.shoulder.y - p.hip.y;
  return Point(
      p.shoulder.x, p.shoulder.y - _armPerTrunk * math.sqrt(dx * dx + dy * dy));
}

/// The fit that keeps every frame of [range] whole on a canvas of [size]:
/// one scale, so the figure does not jump as it plays. [range] must not be
/// empty.
FigureFit fitRange(List<RangeFrame> range, Size size) {
  final xs = <double>[0], ys = <double>[0];
  void add(Point p, [double r = 0]) {
    xs.addAll([p.x - r, p.x + r]);
    ys.addAll([p.y - r, p.y + r]);
  }

  for (final f in range) {
    final p = f.posture;
    [p.ankle, p.knee, p.hip, p.shoulder].forEach(add);
    add(_head(p), _headRadiusM);
    add(Point(p.ankle.x - _heelM, 0));
    add(Point(p.ankle.x + _toesM, 0));
    final load = p.load;
    load == null ? add(_hangingHand(p)) : add(load.at, plateRadiusM);
  }
  final (minX, maxX) = (xs.reduce(math.min), xs.reduce(math.max));
  final (minY, maxY) = (ys.reduce(math.min), ys.reduce(math.max));
  final (w, h) = (maxX - minX, maxY - minY);
  final scale =
      math.min((size.width - 2 * _padPx) / w, (size.height - 2 * _padPx) / h);
  return FigureFit(
    scale,
    Offset((size.width - w * scale) / 2 - minX * scale,
        (size.height + h * scale) / 2 + minY * scale),
  );
}

double _maxGroundN(List<RangeFrame> range) =>
    range.map((f) => f.groundN).reduce(math.max);

/// Newtons per logical pixel for every vector of [range]: its largest
/// `ground_n` spans [maxVectorFraction] of the height. [range] must not be
/// empty.
double newtonsPerPixel(List<RangeFrame> range, Size size) =>
    _maxGroundN(range) / (maxVectorFraction * size.height);

/// The figure for [current] over [range] (the frames it plays between) on
/// a canvas of [size]. The posture drawn is [current]'s, interpolated or
/// not; the forces and the hip moment heating the bar's arrow are its
/// returned frame's.
FigureGeometry figureGeometry(
    List<RangeFrame> range, PlaybackFrame current, Size size) {
  final fit = fitRange(range, size);
  final nPerPx = newtonsPerPixel(range, size);
  final maxMoment = maxAbsMoment(range);
  final p = current.posture;
  final c = fit.toCanvas;

  final midFoot = c(const Point(0, 0));
  final shoulder = c(p.shoulder), hip = c(p.hip);
  final load = p.load;
  final bar = load == null ? null : c(load.at);
  final barN = current.frame.hip?.barN;
  final groundN = current.frame.groundN;
  final hipNm = current.frame.mechanics.moment(Joint.hip);
  final barHeat = maxMoment == 0 ? 0.0 : math.max(0.0, hipNm) / maxMoment;
  // The arrow hangs from under the plate, as in design v2, so the plate
  // does not hide where it starts.
  final barFrom =
      load == null ? null : c(Point(load.at.x, load.at.y - plateRadiusM));

  return FigureGeometry(
    scale: fit.scale,
    groundY: midFoot.dy,
    ankle: c(p.ankle),
    knee: c(p.knee),
    hip: hip,
    shoulder: shoulder,
    head: c(_head(p)),
    headRadius: _headRadiusM * fit.scale,
    foot: (c(Point(p.ankle.x - _heelM, 0)), c(Point(p.ankle.x + _toesM, 0))),
    arm: load == null
        ? [shoulder, c(_hangingHand(p))]
        : [shoulder, c(_add(load.at, _elbowFromBar)), c(load.at)],
    bar: bar,
    plateRadius: plateRadiusM * fit.scale,
    balanceLine: (midFoot, Offset(midFoot.dx, _padPx)),
    lever: bar == null ? null : (hip, Offset(bar.dx, hip.dy)),
    ground: ForceVector(midFoot, midFoot.translate(0, -groundN / nPerPx), null),
    barForce: barFrom == null || barN == null
        ? null
        : ForceVector(barFrom, barFrom.translate(0, barN / nPerPx), barHeat),
  );
}
