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
}
