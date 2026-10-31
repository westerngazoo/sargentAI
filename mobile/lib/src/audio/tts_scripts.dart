import '../../l10n/app_localizations.dart';

/// The earbud coach's spoken lines, from the catalogue (R-0061 AC11). Each
/// takes the catalogue because the coach runs outside the widget tree.
abstract final class TtsScripts {
  /// The line that opens an exercise: its name and, when known, its sets,
  /// reps and weight.
  static String exerciseStart(
    AppLocalizations l10n,
    String name,
    int? sets,
    int? reps,
    double? weight,
  ) {
    if (sets == null || reps == null) return l10n.ttsNext(name);
    if (weight != null) {
      return l10n.ttsNextSetsWeight(name, sets, reps, _kg(weight));
    }
    return l10n.ttsNextSets(name, sets, reps);
  }

  /// The line that opens a set: its number and, when known, the total,
  /// reps and weight.
  static String setStart(
    AppLocalizations l10n,
    int setNumber,
    int? totalSets,
    int? reps,
    double? weight,
  ) {
    if (totalSets == null || reps == null) return l10n.ttsSetGo(setNumber);
    if (weight != null) {
      return l10n.ttsSetOfWeightGo(setNumber, totalSets, reps, _kg(weight));
    }
    return l10n.ttsSetOfGo(setNumber, totalSets, reps);
  }

  static String rest(AppLocalizations l10n) => l10n.ttsRest;
  static String workoutDone(AppLocalizations l10n) => l10n.ttsWorkoutDone;
  static String seedFirstSet(AppLocalizations l10n) => l10n.ttsSeedFirstSet;

  /// "100" for a whole number of kilos, "62.5" otherwise.
  static String _kg(double weight) =>
      weight.toStringAsFixed(1).replaceAll(RegExp(r'\.0$'), '');
}
