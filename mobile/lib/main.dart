import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';
import 'src/core/theme/font_licences.dart';

void main() {
  registerFontLicences();
  runApp(const ProviderScope(child: FitAiApp()));
}
