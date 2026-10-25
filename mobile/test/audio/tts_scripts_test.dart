// R-0061 AC11: the earbud coach's spoken scripts come from the catalogue.

import 'package:fitai/src/audio/tts_scripts.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  test('exerciseStart speaks the catalogue line for each shape', () {
    expect(TtsScripts.exerciseStart(l10n, 'Squat', null, null, null),
        l10n.ttsNext('Squat'));
    expect(TtsScripts.exerciseStart(l10n, 'Squat', 3, 5, null),
        l10n.ttsNextSets('Squat', 3, 5));
    expect(TtsScripts.exerciseStart(l10n, 'Squat', 3, 5, 100.0),
        l10n.ttsNextSetsWeight('Squat', 3, 5, '100'));
    expect(TtsScripts.exerciseStart(l10n, 'Squat', 3, 5, 62.5),
        l10n.ttsNextSetsWeight('Squat', 3, 5, '62.5'));
  });

  test('setStart speaks the catalogue line for each shape', () {
    expect(TtsScripts.setStart(l10n, 2, null, null, null), l10n.ttsSetGo(2));
    expect(TtsScripts.setStart(l10n, 2, 3, 5, null), l10n.ttsSetOfGo(2, 3, 5));
    expect(TtsScripts.setStart(l10n, 2, 3, 5, 80.0),
        l10n.ttsSetOfWeightGo(2, 3, 5, '80'));
  });

  test('the fixed lines are catalogue entries', () {
    expect(TtsScripts.rest(l10n), l10n.ttsRest);
    expect(TtsScripts.workoutDone(l10n), l10n.ttsWorkoutDone);
    expect(TtsScripts.seedFirstSet(l10n), l10n.ttsSeedFirstSet);
  });
}
