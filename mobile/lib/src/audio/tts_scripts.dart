class TtsScripts {
  static String exerciseStart(String name, int sets, int reps, double? weight) {
    if (weight != null) {
      return 'Next: $name. $sets sets of $reps reps at ${weight.toStringAsFixed(1).replaceAll(RegExp(r'\.0$'), '')} kg.';
    } else {
      return 'Next: $name. $sets sets of $reps reps.';
    }
  }

  static String setStart(int setNumber, int totalSets, int reps, double? weight) {
    if (weight != null) {
      return 'Set $setNumber of $totalSets. $reps reps at ${weight.toStringAsFixed(1).replaceAll(RegExp(r'\.0$'), '')} kg. Go.';
    } else {
      return 'Set $setNumber of $totalSets. $reps reps. Go.';
    }
  }

  static const String rest = 'Rest.';
  static const String workoutDone = 'Workout done.';
}
