// SAC3 -> AC3 (domain): SetDraft validators mirror `core::workout` EXACTLY at
// every boundary —
//   reps      required int, [1, 10 000]          (Reps::try_new)
//   weight_kg optional finite double, (0, 1000]  (LoadKg::try_new — 0 EXCLUSIVE)
//   rpe       optional [6.0, 10.0] in exact 0.5 steps (Rpe::try_new)
// Each validator returns `String?` (null = ok), the ProfileDraft idiom; the
// messages are user-safe inline copy, so only nullability is pinned here.
//
// RED until package:fitai/src/workout/domain/set_draft.dart defines SetDraft
// with repsError()/weightError()/rpeError()/valid.

import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC3 repsError — required int in [1, 10000]', () {
    test('missing reps is rejected (required)', () {
      expect(const SetDraft().repsError(), isNotNull);
    });

    test('lower boundary: 0 rejected, 1 accepted', () {
      expect(const SetDraft(reps: 0).repsError(), isNotNull);
      expect(const SetDraft(reps: 1).repsError(), isNull);
    });

    test('upper boundary: 10000 accepted, 10001 rejected', () {
      expect(const SetDraft(reps: 10000).repsError(), isNull);
      expect(const SetDraft(reps: 10001).repsError(), isNotNull);
    });

    test('negative reps rejected', () {
      expect(const SetDraft(reps: -1).repsError(), isNotNull);
    });
  });

  group('SAC3 weightError — optional finite double in (0, 1000]', () {
    test('absent weight is fine (optional)', () {
      expect(const SetDraft(reps: 5).weightError(), isNull);
    });

    test('zero is rejected — the lower bound is EXCLUSIVE', () {
      expect(const SetDraft(reps: 5, weightKg: 0).weightError(), isNotNull);
    });

    test('a small fractional load is accepted (a 0.25 kg microplate)', () {
      expect(const SetDraft(reps: 5, weightKg: 0.25).weightError(), isNull);
    });

    test('upper boundary: 1000.0 accepted, 1000.1 rejected', () {
      expect(const SetDraft(reps: 5, weightKg: 1000.0).weightError(), isNull);
      expect(
        const SetDraft(reps: 5, weightKg: 1000.1).weightError(),
        isNotNull,
      );
    });

    test('negative weight rejected', () {
      expect(const SetDraft(reps: 5, weightKg: -20).weightError(), isNotNull);
    });

    test('non-finite weight rejected (infinity, NaN)', () {
      expect(
        const SetDraft(reps: 5, weightKg: double.infinity).weightError(),
        isNotNull,
      );
      expect(
        const SetDraft(reps: 5, weightKg: double.nan).weightError(),
        isNotNull,
      );
    });
  });

  group('SAC3 rpeError — optional [6.0, 10.0] in exact 0.5 steps', () {
    test('absent RPE is fine (optional)', () {
      expect(const SetDraft(reps: 5).rpeError(), isNull);
    });

    test('boundaries: 6.0 and 10.0 accepted; 5.5 and 10.5 rejected', () {
      expect(const SetDraft(reps: 5, rpe: 6.0).rpeError(), isNull);
      expect(const SetDraft(reps: 5, rpe: 10.0).rpeError(), isNull);
      expect(const SetDraft(reps: 5, rpe: 5.5).rpeError(), isNotNull);
      expect(const SetDraft(reps: 5, rpe: 10.5).rpeError(), isNotNull);
    });

    test('the 0.5 grid is exact: 7.5 accepted, 7.3 rejected (SAC3 pin)', () {
      expect(const SetDraft(reps: 5, rpe: 7.5).rpeError(), isNull);
      expect(const SetDraft(reps: 5, rpe: 7.3).rpeError(), isNotNull);
    });

    test('an in-range off-grid value is rejected (6.25)', () {
      expect(const SetDraft(reps: 5, rpe: 6.25).rpeError(), isNotNull);
    });

    test('non-finite RPE rejected (infinity, NaN)', () {
      expect(
        const SetDraft(reps: 5, rpe: double.infinity).rpeError(),
        isNotNull,
      );
      expect(const SetDraft(reps: 5, rpe: double.nan).rpeError(), isNotNull);
    });
  });

  group('SAC3 valid — all three validators pass', () {
    test('reps-only set is valid (optionals absent)', () {
      expect(const SetDraft(reps: 8).valid, isTrue);
    });

    test('fully-specified in-range set is valid', () {
      expect(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5).valid, isTrue);
    });

    test('any failing field makes the set invalid', () {
      expect(const SetDraft().valid, isFalse);
      expect(const SetDraft(reps: 0).valid, isFalse);
      expect(const SetDraft(reps: 8, weightKg: 0).valid, isFalse);
      expect(const SetDraft(reps: 8, rpe: 7.3).valid, isFalse);
    });
  });
}
