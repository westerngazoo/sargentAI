// AC8/AC9 (widget): the home recent-sessions list renders the backlog (date +
// exercise/set counts), shows the empty state, and deletes after a confirm
// dialog. Fills the coverage gap qa flagged at step-7 sign-off (the controller
// and provider were unit-tested; this pumps the widget).

import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/presentation/session_list.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerWorkoutFallbacks);

  late MockWorkoutRepository repo;

  setUp(() {
    repo = MockWorkoutRepository();
  });

  Future<ProviderContainer> pumpList(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [workoutRepositoryProvider.overrideWithValue(repo)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: SessionList())),
      ),
    );
    await tester.pump(); // resolve the workoutsProvider future
    await tester.pump();
    return container;
  }

  testWidgets('AC8: an empty backlog shows the empty state', (tester) async {
    when(() => repo.list()).thenAnswer((_) async => []);
    await pumpList(tester);

    expect(find.textContaining('no workouts yet'), findsOneWidget);
  });

  testWidgets('AC8: a session renders with its exercise and set counts',
      (tester) async {
    when(() => repo.list()).thenAnswer((_) async => [
          sampleSession(exercises: [
            exerciseResponseJson(sets: [
              setResponseJson(id: 'a', position: 1),
              setResponseJson(id: 'b', position: 2),
            ]),
          ]),
        ]);
    await pumpList(tester);

    expect(find.textContaining('1 exercises'), findsOneWidget);
    expect(find.textContaining('2 sets'), findsOneWidget);
  });

  testWidgets('AC9: delete asks for confirmation, then calls DELETE',
      (tester) async {
    when(() => repo.list()).thenAnswer((_) async => [sampleSession(id: 's-1')]);
    when(() => repo.delete('s-1')).thenAnswer((_) async {});
    await pumpList(tester);

    await tester.tap(find.byTooltip('Delete'));
    await tester.pump();
    expect(find.text('Delete this workout?'), findsOneWidget);

    await tester.tap(find.text('Delete').last);
    await tester.pump();
    await tester.pump();

    verify(() => repo.delete('s-1')).called(1);
  });

  testWidgets('AC9: cancelling the confirm does NOT delete', (tester) async {
    when(() => repo.list()).thenAnswer((_) async => [sampleSession(id: 's-1')]);
    await pumpList(tester);

    await tester.tap(find.byTooltip('Delete'));
    await tester.pump();
    await tester.tap(find.text('Cancel'));
    await tester.pump();

    verifyNever(() => repo.delete(any()));
  });
}
