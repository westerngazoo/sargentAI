// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTagline => 'The physics of your lift — measured, not guessed.';

  @override
  String get emailLabel => 'Email';

  @override
  String get passwordLabel => 'Password';

  @override
  String get logIn => 'Log in';

  @override
  String get createAnAccount => 'Create an account';

  @override
  String get forgotPassword => 'Forgot password?';

  @override
  String get continueWithGoogle => 'Continue with Google';

  @override
  String get useTestAccount => 'Use test account';

  @override
  String get createYourAccount => 'Create your account';

  @override
  String get createAccount => 'Create account';

  @override
  String get backToLogIn => 'Back to log in';

  @override
  String get emailAlreadyRegistered =>
      'That email already has an account. Log in instead.';

  @override
  String get resetYourPassword => 'Reset your password';

  @override
  String get sendResetLink => 'Send reset link';

  @override
  String get resetLinkSent =>
      'If that address has an account, a reset link is on its way. It is valid for 30 minutes.';

  @override
  String get setANewPassword => 'Set a new password';

  @override
  String get newPasswordLabel => 'New password';

  @override
  String get confirmNewPasswordLabel => 'Confirm new password';

  @override
  String get setNewPassword => 'Set new password';

  @override
  String get passwordsDoNotMatch => 'The passwords don\'t match.';

  @override
  String get useAtLeast8Characters => 'Use at least 8 characters.';

  @override
  String get passwordUpdated => 'Password updated. Log in with it.';

  @override
  String get linkNoLongerValid =>
      'This link is no longer valid. Links expire after 30 minutes and work once.';

  @override
  String get requestANewLink => 'Request a new link';

  @override
  String get setUpYourProfile => 'Set up your profile';

  @override
  String get onboardingMissingBodyStats =>
      'Add your date of birth, height and weight to continue.';

  @override
  String get onboardingMissingGoals => 'Pick at least one goal to continue.';

  @override
  String get back => 'Back';

  @override
  String get next => 'Next';

  @override
  String get finish => 'Finish';

  @override
  String get completeYourProfilePrompt =>
      'Complete your profile to personalize your training.';

  @override
  String get getStarted => 'Get started';

  @override
  String get dismiss => 'Dismiss';

  @override
  String get aboutYou => 'About you';

  @override
  String get dateOfBirthLabel => 'Date of birth';

  @override
  String get notSet => 'Not set';

  @override
  String get choose => 'Choose';

  @override
  String get heightCmLabel => 'Height (cm)';

  @override
  String get weightKgLabel => 'Weight (kg)';

  @override
  String get dobMissing => 'enter your date of birth';

  @override
  String get dobInFuture => 'date of birth cannot be in the future';

  @override
  String ageOutOfRange(int min, int max) {
    return 'age must be between $min and $max';
  }

  @override
  String get heightMissing => 'enter your height';

  @override
  String heightOutOfRange(int min, int max) {
    return 'height must be between $min and $max cm';
  }

  @override
  String get weightMissing => 'enter your weight';

  @override
  String weightOutOfRange(int min, int max) {
    return 'weight must be between $min and $max kg';
  }

  @override
  String bodyFatOutOfRange(int min, int max) {
    return 'body fat must be between $min and $max %';
  }

  @override
  String get yourGoals => 'Your goals';

  @override
  String get pickAtLeastOne => 'Pick at least one.';

  @override
  String get goalLoseFat => 'Lose fat';

  @override
  String get goalBuildMuscle => 'Build muscle';

  @override
  String get goalRecomp => 'Recomposition';

  @override
  String get goalMaintain => 'Maintain';

  @override
  String get goalGainStrength => 'Gain strength';

  @override
  String get optionalDetails => 'Optional details';

  @override
  String get youCanSkipThese => 'You can skip these.';

  @override
  String get sexMale => 'Male';

  @override
  String get sexFemale => 'Female';

  @override
  String get clear => 'Clear';

  @override
  String get bodyFatOptionalLabel => 'Body fat % (optional)';

  @override
  String get chooseYourBodyType => 'Choose your body type';

  @override
  String get bodyShape => 'Body shape';

  @override
  String get bodyFatLevel => 'Body fat level';

  @override
  String get findMyProgram => 'Find my program';

  @override
  String get bodyShapeEctomorph => 'Lean & Narrow';

  @override
  String get bodyShapeMesomorph => 'Athletic & Broad';

  @override
  String get bodyShapeEndomorph => 'Stocky & Solid';

  @override
  String get bodyShapeEctomorphDescription =>
      'Slender frame, narrow shoulders, long limbs, low body mass.';

  @override
  String get bodyShapeMesomorphDescription =>
      'Wide shoulders, muscular build, responds well to training.';

  @override
  String get bodyShapeEndomorphDescription =>
      'Stocky build, broad waist, tends to carry more mass.';

  @override
  String get fatBandLean => 'Lean';

  @override
  String get fatBandModerate => 'Moderate';

  @override
  String get fatBandBulky => 'Bulky';

  @override
  String get fatBandLeanDescription => 'Visible muscle definition';

  @override
  String get fatBandModerateDescription => 'Some definition, moderate fat';

  @override
  String get fatBandBulkyDescription => 'Higher body fat, full look';

  @override
  String get yourProgram => 'Your program';

  @override
  String get couldNotLoadYourProgram => 'Could not load your program';

  @override
  String get retry => 'Retry';

  @override
  String get noProgramYet => 'No program yet';

  @override
  String get training => 'Training';

  @override
  String get split => 'Split';

  @override
  String daysPerWeek(int n) {
    return '$n days/week';
  }

  @override
  String get sessionDuration => 'Session duration';

  @override
  String minutesShort(int n) {
    return '$n min';
  }

  @override
  String get intensity => 'Intensity';

  @override
  String get rest => 'Rest';

  @override
  String get progression => 'Progression';

  @override
  String get highlightExercises => 'Highlight exercises';

  @override
  String get nutrition => 'Nutrition';

  @override
  String get approach => 'Approach';

  @override
  String get strategy => 'Strategy';

  @override
  String get meals => 'Meals';

  @override
  String get protein => 'Protein';

  @override
  String get carbs => 'Carbs';

  @override
  String get fat => 'Fat';

  @override
  String grams(int n) {
    return '$n g';
  }

  @override
  String get kcalPerDay => 'kcal / day';

  @override
  String get targetPhysiqueCaps => 'TARGET PHYSIQUE';

  @override
  String get physiqueClassicAesthetic => 'Classic aesthetic';

  @override
  String get physiqueModernHypertrophy => 'Modern hypertrophy';

  @override
  String get physiqueLeanMinimalist => 'Lean minimalist';

  @override
  String get physiquePowerbuilder => 'Powerbuilder';

  @override
  String get physiqueHeavyDutyMass => 'Heavy-duty mass';

  @override
  String get physiqueMassMonster => 'Mass monster';

  @override
  String get physiqueClassicAestheticBlurb =>
      'Wide shoulders, small waist — the golden-era X-frame.';

  @override
  String get physiqueModernHypertrophyBlurb =>
      'Balanced, detailed, athletic — proportion over pure size.';

  @override
  String get physiqueLeanMinimalistBlurb =>
      'Lean and hard — dense muscle, minimal excess.';

  @override
  String get physiquePowerbuilderBlurb =>
      'Thick and powerful — a strongman-meets-bodybuilder build.';

  @override
  String get physiqueHeavyDutyMassBlurb =>
      'Dense, heavy-duty mass — maximum muscle, blocky and strong.';

  @override
  String get physiqueMassMonsterBlurb =>
      'Maximum size — the mass-monster silhouette.';

  @override
  String get physiqueDefaultBlurb => 'Your program\'s target build.';

  @override
  String taperTag(String ratio) {
    return 'Taper $ratio:1  ⓘ';
  }

  @override
  String get physiqueMass => 'Mass';

  @override
  String get physiqueLean => 'Lean';

  @override
  String get physiqueBalanced => 'Balanced';

  @override
  String get taperByBuild => 'Taper by build';

  @override
  String get taperTableExplainer =>
      'Shoulder-to-waist of the target silhouette. A higher number is a more dramatic V. It compares these builds with each other — it is not a measurement of your body.';

  @override
  String taperRatio(String ratio) {
    return '$ratio:1';
  }

  @override
  String get thisWeekCaps => 'THIS WEEK';

  @override
  String daysOfTarget(int done, int target) {
    return '$done of $target days';
  }

  @override
  String sessionsLoggedOnProgram(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sessions logged on this program',
      one: '1 session logged on this program',
    );
    return '$_temp0';
  }

  @override
  String get targetMet => 'Target met';

  @override
  String percent(int n) {
    return '$n%';
  }

  @override
  String get chooseYourProgram => 'Choose your program';

  @override
  String get couldNotLoadProposals => 'Could not load proposals';

  @override
  String get rankBestMatch => 'Best match';

  @override
  String get rankCloseMatch => 'Close match';

  @override
  String get rankGoodOption => 'Good option';

  @override
  String proposalSummaryLine(int days, int kcal) {
    return '$days days/week · $kcal kcal/day';
  }

  @override
  String intensityLine(String value) {
    return 'Intensity: $value';
  }

  @override
  String restLine(String value) {
    return 'Rest: $value';
  }

  @override
  String progressionLine(String value) {
    return 'Progression: $value';
  }

  @override
  String get chooseThisProgram => 'Choose this program';

  @override
  String get selectionNoLongerAvailable =>
      'Selection no longer available — please refresh.';

  @override
  String get couldNotChooseProgram =>
      'Could not choose program — please retry.';

  @override
  String get profileMenu => 'Your profile';

  @override
  String get menuProgram => 'Program';

  @override
  String get menuMeasurements => 'Measurements';

  @override
  String get menuProgress => 'Progress';

  @override
  String get signOut => 'Sign out';

  @override
  String get dictate => 'Dictate';

  @override
  String get today => 'Today';

  @override
  String get dayDoneHeadline => 'You trained';

  @override
  String get dayFreeHeadline => 'Free session';

  @override
  String get startSession => 'Start session';

  @override
  String get logLate => 'Log late';

  @override
  String get viewLogged => 'See what you logged';

  @override
  String get nothingLogged => 'Nothing logged this day.';

  @override
  String get nutritionToday => 'Nutrition · today';

  @override
  String get addMeal => '+ Meal';

  @override
  String get showMonth => 'Show month';

  @override
  String get showWeek => 'Show week';

  @override
  String get findYourProgram => 'Find your program';

  @override
  String get apiErrorTransport => 'can\'t reach the server — retry';

  @override
  String get apiErrorBadRequest => 'please check your details';

  @override
  String get apiErrorUnauthorized => 'invalid email or password';

  @override
  String get apiErrorNotFound => 'that no longer exists';

  @override
  String get apiErrorConflict => 'that already exists';

  @override
  String get apiErrorUnknown => 'something went wrong — please try again';

  @override
  String get regionNeck => 'Neck';

  @override
  String get regionTraps => 'Traps';

  @override
  String get regionShoulders => 'Shoulders';

  @override
  String get regionChest => 'Chest';

  @override
  String get regionBiceps => 'Biceps';

  @override
  String get regionTriceps => 'Triceps';

  @override
  String get regionForearms => 'Forearms';

  @override
  String get regionCore => 'Core';

  @override
  String get regionLats => 'Lats';

  @override
  String get regionUpperBack => 'Upper back';

  @override
  String get regionLowerBack => 'Lower back';

  @override
  String get regionQuads => 'Quads';

  @override
  String get regionHamstrings => 'Hamstrings';

  @override
  String get regionGlutes => 'Glutes';

  @override
  String get regionCalves => 'Calves';

  @override
  String get muscleChest => 'Chest';

  @override
  String get muscleBack => 'Back';

  @override
  String get muscleShoulders => 'Shoulders';

  @override
  String get muscleArms => 'Arms';

  @override
  String get muscleLegs => 'Legs';

  @override
  String get muscleCore => 'Core';

  @override
  String listAnd(String head, String last) {
    return '$head and $last';
  }

  @override
  String get rejectStartWorkoutFirst => 'start a workout first';

  @override
  String get rejectAddExerciseFirst => 'add an exercise first';

  @override
  String get rejectEnterExerciseName => 'enter an exercise name';

  @override
  String rejectNameTooLong(int max) {
    return 'name must be at most $max characters';
  }

  @override
  String get rejectEnterReps => 'enter the reps';

  @override
  String rejectRepsOutOfRange(int min, int max) {
    return 'reps must be between $min and $max';
  }

  @override
  String rejectWeightOutOfRange(int max) {
    return 'weight must be over 0 and at most $max kg';
  }

  @override
  String rejectRpeOutOfRange(double min, double max) {
    return 'RPE must be between $min and $max in 0.5 steps';
  }

  @override
  String get fieldAreaSets => 'your sets';

  @override
  String get fieldAreaExercise => 'the exercise';

  @override
  String get fieldAreaWorkout => 'the workout';

  @override
  String errorCheckArea(String error, String area) {
    return '$error — check $area';
  }

  @override
  String get discardThisWorkout => 'Discard this workout?';

  @override
  String get yourLoggedSetsWillBeLost => 'Your logged sets will be lost.';

  @override
  String get cancel => 'Cancel';

  @override
  String get discard => 'Discard';

  @override
  String get workout => 'Workout';

  @override
  String get earbudCoachOn => 'Earbud coach on';

  @override
  String get earbudCoachOff => 'Earbud coach off';

  @override
  String get voiceCoachOn => 'Voice coach on';

  @override
  String get voiceCoachOff => 'Voice coach off';

  @override
  String get addExercise => 'Add exercise';

  @override
  String targetMusclesCaps(String name) {
    return 'TARGET MUSCLES — $name';
  }

  @override
  String get stopListening => 'Stop listening';

  @override
  String get dictateASet => 'Dictate a set';

  @override
  String get listening => 'Listening…';

  @override
  String get coachIdleHint =>
      'Tap the mic and say \"done\" — I will ask your reps and kilos. End every answer with \"over\".';

  @override
  String setLine(int n, String label) {
    return 'Set $n: $label';
  }

  @override
  String repsCount(int n) {
    return '$n reps';
  }

  @override
  String kg(String value) {
    return '$value kg';
  }

  @override
  String rpeValue(String value) {
    return 'RPE $value';
  }

  @override
  String get repsLabel => 'Reps';

  @override
  String get rpeLabel => 'RPE';

  @override
  String get repeatLastSet => 'Repeat last set';

  @override
  String get logSet => 'Log set';

  @override
  String get newExercise => 'New exercise';

  @override
  String get add => 'Add';

  @override
  String get exerciseNameLabel => 'Exercise name';

  @override
  String get commonLifts => 'Common lifts';

  @override
  String get muscleGroupOptional => 'Muscle group (optional)';

  @override
  String exerciseWithGroup(String name, String group) {
    return '$name · $group';
  }

  @override
  String get noWorkoutsYet =>
      'no workouts yet — tap start to log your first one';

  @override
  String sessionSummary(int exercises, int sets) {
    return '$exercises exercises · $sets sets';
  }

  @override
  String get delete => 'Delete';

  @override
  String get deleteThisWorkout => 'Delete this workout?';

  @override
  String get yesterday => 'Yesterday';

  @override
  String get workoutNoLongerExists => 'that workout no longer exists';

  @override
  String get startWorkout => 'Start workout';

  @override
  String get coachSayDoneHandsFree => 'Say done when your set is in.';

  @override
  String get coachSayDoneKeyMic =>
      'Key the mic and say done when your set is in.';

  @override
  String coachTargetSuffix(String label) {
    return ' — target $label';
  }

  @override
  String coachPlanLoaded(int count, String first, String prompt) {
    return 'Plan loaded — $count exercises. First up: $first. $prompt';
  }

  @override
  String get coachNoPlan =>
      'Voice coach on. No plan found — add an exercise, then dictate your sets.';

  @override
  String coachOnCurrent(String name, String prompt) {
    return 'Voice coach on. Current exercise: $name. $prompt';
  }

  @override
  String get coachNoCurrentExercise => 'none';

  @override
  String get voiceInputUnavailable => 'Voice input is not available here.';

  @override
  String get coachHowManyReps => 'How many reps?';

  @override
  String get coachHowManyKilos => 'How many kilos? Or say bodyweight.';

  @override
  String coachLoggedRestUp(String logged) {
    return 'Logged $logged. Rest up.';
  }

  @override
  String coachAtKilos(String weight) {
    return 'at $weight kilos';
  }

  @override
  String coachNextUp(String next) {
    return 'Next up: $next.';
  }

  @override
  String get coachLastExercise =>
      'That was the last exercise. Say finish workout to save.';

  @override
  String get coachSaving => 'Saving your workout.';

  @override
  String get coachSaved => 'Workout saved. Dismissed!';

  @override
  String get coachGoodHowManyReps => 'Good. How many reps?';

  @override
  String get coachStandingBy => 'Standing by. Tap the mic when you are ready.';

  @override
  String get coachDidNotCatch =>
      'Did not catch that. Say for example: ten reps at sixty kilos.';

  @override
  String get coachJustReps => 'Just the number of reps.';

  @override
  String get coachKilosAsNumber => 'Kilos as a number — or say bodyweight.';

  @override
  String coachLoggedNextSet(String logged) {
    return 'Logged $logged. Rest up — say done when the next set is in.';
  }

  @override
  String ttsNext(String name) {
    return 'Next: $name.';
  }

  @override
  String ttsNextSets(String name, int sets, int reps) {
    return 'Next: $name. $sets sets of $reps reps.';
  }

  @override
  String ttsNextSetsWeight(String name, int sets, int reps, String weight) {
    return 'Next: $name. $sets sets of $reps reps at $weight kg.';
  }

  @override
  String ttsSetGo(int n) {
    return 'Set $n. Go.';
  }

  @override
  String ttsSetOfGo(int n, int total, int reps) {
    return 'Set $n of $total. $reps reps. Go.';
  }

  @override
  String ttsSetOfWeightGo(int n, int total, int reps, String weight) {
    return 'Set $n of $total. $reps reps at $weight kg. Go.';
  }

  @override
  String get ttsRest => 'Rest.';

  @override
  String get ttsWorkoutDone => 'Workout done.';

  @override
  String get ttsSeedFirstSet => 'Log your first set on screen.';

  @override
  String get sergeantVoiceUnavailable =>
      'Voice input is not available here — tap an option instead.';

  @override
  String get sergeantGreeting =>
      'Sergeant here. Say: start workout, plan workout, log a meal. Finish every command with over.';

  @override
  String get sergeantRogerOut => 'Roger. Out.';

  @override
  String get sergeantLogged => 'Logged.';

  @override
  String get sergeantTellMeMore => 'Tell me more.';

  @override
  String get sergeantRoger => 'Roger.';

  @override
  String get sergeantDidNotCatch => 'Did not catch that.';

  @override
  String get sergeantStandingBy => 'Standing by.';

  @override
  String get sergeantAskMacros =>
      'Tell me the grams of protein, carbs, and fat — a portion like 200 grams of chicken breast — or a preset, like protein shake.';

  @override
  String get sergeantStartingSession => 'Starting your session.';

  @override
  String get sergeantNoPlanYet => 'No plan yet — let us find your body match.';

  @override
  String get sergeantOpeningBodyMatch => 'Opening the body match.';

  @override
  String get sergeantRecentActivity => 'Your recent activity.';

  @override
  String get sergeantOpeningProfile => 'Opening your profile.';

  @override
  String get sergeantSayOptions =>
      'Say: start workout, plan workout, or log a meal.';

  @override
  String sergeantDidNotGet(String transcript) {
    return 'Did not get \"$transcript\" — say start workout, plan workout, or log a meal.';
  }

  @override
  String get sergeantMacrosExample =>
      'Grams of protein, carbs, and fat — for example: 40 protein, 60 carbs, 20 fat.';

  @override
  String get sergeantMealLogged => 'Meal logged';

  @override
  String sergeantLoggedLabel(String label) {
    return 'Logged $label';
  }

  @override
  String sergeantMealSummary(
      String what, String protein, String carbs, String fat, int kcal) {
    return '$what: $protein protein, $carbs carbs, $fat fat — $kcal calories. What else?';
  }

  @override
  String get sergeantCouldNotSaveMeal =>
      'Could not save the meal — try again in a moment.';

  @override
  String sergeantLookingUp(String food) {
    return 'Looking up $food.';
  }

  @override
  String sergeantCouldNotFind(String food) {
    return 'Could not find $food — tell me the grams of protein, carbs, and fat instead.';
  }

  @override
  String sergeantGramsOf(String grams, String food) {
    return '$grams grams of $food';
  }

  @override
  String sergeantPlanSummary(String split, int days, int kcal) {
    return 'Your plan: $split. $days days a week, $kcal calories.';
  }

  @override
  String get hubWorkout => 'Workout';

  @override
  String get hubMeal => 'Meal';

  @override
  String get hubProgram => 'Program';

  @override
  String get hubBodyMatch => 'Body match';

  @override
  String get hubHistory => 'History';

  @override
  String get hubProfile => 'Profile';

  @override
  String get hubIdleHint =>
      'Tap the mic and speak — finish every command with \"over\".';

  @override
  String get progress => 'Progress';

  @override
  String get logout => 'Logout';

  @override
  String get logMeasurement => 'Log measurement';

  @override
  String get bodyComposition => 'Body composition';

  @override
  String get strength => 'Strength';

  @override
  String get trainingBalance => 'Training balance';

  @override
  String get couldNotLoadPullToRetry => 'Could not load — pull to retry.';

  @override
  String get logACoupleOfMeasurements =>
      'Log a couple of measurements to see your trend.';

  @override
  String get bodyFatPercentLabel => 'Body fat %';

  @override
  String get leanMass => 'Lean mass';

  @override
  String get weight => 'Weight';

  @override
  String get bodyFat => 'Body fat';

  @override
  String overTime(String label) {
    return '$label over time';
  }

  @override
  String get logAFewWeightedSessions =>
      'Log a few weighted sessions to see your strength climb.';

  @override
  String get estimatedOneRmAllLifts => 'Estimated 1RM · all lifts';

  @override
  String estimatedOneRm(String name) {
    return 'Estimated 1RM · $name';
  }

  @override
  String get allLifts => 'All lifts';

  @override
  String gainSinceTracking(String gain) {
    return '$gain kg since you started tracking it';
  }

  @override
  String get e1rmLabel => 'e1RM';

  @override
  String get sessionVolume => 'Session volume';

  @override
  String get volume => 'Volume';

  @override
  String get prBadge => 'PR';

  @override
  String get coachSuggestions => 'Coach suggestions';

  @override
  String lastWeeks(int n) {
    return 'last $n wks';
  }

  @override
  String get onTrack => 'On track — keep doing what you\'re doing.';

  @override
  String get actionCaps => 'ACTION';

  @override
  String get measurementLogged => 'Measurement logged';

  @override
  String get weighInHint =>
      'Weigh in — body fat % is optional but powers the trend.';

  @override
  String get bodyFatPercentInputLabel => 'Body fat (%)';

  @override
  String get saveMeasurement => 'Save measurement';

  @override
  String get mealLogged => 'Meal logged';

  @override
  String get logAMeal => 'Log a meal';

  @override
  String get proteinGramsLabel => 'Protein (g)';

  @override
  String get carbsGramsLabel => 'Carbs (g)';

  @override
  String get fatGramsLabel => 'Fat (g)';

  @override
  String kcalLoggedForToday(String kcal) {
    return '≈ $kcal kcal · logged for today';
  }

  @override
  String get saveMeal => 'Save meal';

  @override
  String valueWithUnit(String value, String unit) {
    return '$value$unit';
  }
}
