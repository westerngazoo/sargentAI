import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';
import 'src/core/theme/app_theme.dart';

void main() {
  registerFontLicences();
  runApp(const ProviderScope(child: FitAiApp()));
}
