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
  String bodyFatOutOfRange(num min, num max) {
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
  String get yourProgramCaps => 'YOUR PROGRAM';

  @override
  String kcalShort(int n) {
    return '$n kcal';
  }

  @override
  String doneOfTarget(int done, int target) {
    return '$done/$target';
  }

  @override
  String get getYourProgram => 'Get your program';

  @override
  String get takeAPhotoForPlan => 'Take a photo to get a personalized plan';

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
}
