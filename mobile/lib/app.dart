import 'package:flutter/material.dart';
import 'screens/home_screen.dart';

class FitAiApp extends StatelessWidget {
  const FitAiApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'fitAI',
      home: const HomeScreen(),
    );
  }
}
