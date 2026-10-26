// R-0061 AC1, AC8 -> SPEC-0061 §2.6: the Programa panel and its nested
// choose flow. Back and Esc step back within the flow and close the panel
// only from its first page; a tap outside and ✕ close the whole panel from
// any page (N11). Choosing a program completes inside the panel: it sends
// the client's local date as `starts_on`, and the detail re-reads the
// program.

import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/dates.dart';
import 'package:fitai/src/core/panels/show_panel.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/program/models/synthetic_match.dart';
import 'package:fitai/src/program/presentation/body_type_names.dart';
import 'package:fitai/src/program/presentation/body_type_picker_page.dart';
import 'package:fitai/src/program/presentation/program_detail_page.dart';
import 'package:fitai/src/program/presentation/program_panel.dart';
import 'package:fitai/src/program/presentation/synthetic_proposals_page.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/program_fakes.dart';

const phone = Size(390, 844);
const desktop = Size(1440, 900);

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProgramService programService;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    programService = MockProgramService();
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
    whenGetSchedule(programService)
        .thenAnswer((_) async => sampleSchedule(programId: null));
  });

  /// A host whose button opens the Programa panel, as the profile menu does.
  Future<void> pumpPanel(WidgetTester tester, Size size) async {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: localizedApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: Center(
                child: ElevatedButton(
                  onPressed: () => showPanel<void>(
                    context,
                    title: l10n.yourProgram,
                    scrolls: false,
                    body: (_) => const ProgramPanel(),
                  ),
                  child: const Text('open'),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.byType(ProgramDetailPage), findsOneWidget);
  }

  /// From the detail's empty state into the picker, still inside the panel.
  Future<void> toPicker(WidgetTester tester) async {
    expect(find.text(l10n.noProgramYet), findsOneWidget);
    await tester.tap(find.byKey(const Key('get-program')));
    await tester.pumpAndSettle();
    expect(find.byType(BodyTypePickerPage), findsOneWidget);
    expect(find.byKey(const Key('panel-close')), findsOneWidget);
  }

  Future<void> systemBack(WidgetTester tester) async {
    final dynamic widgetsAppState = tester.state(find.byType(WidgetsApp));
    await widgetsAppState.didPopRoute();
    await tester.pumpAndSettle();
  }

  for (final (name, size) in [('phone', phone), ('desktop', desktop)]) {
    group('AC1 on $name', () {
      testWidgets('back steps from the picker to the detail, then closes',
          (tester) async {
        await pumpPanel(tester, size);
        await toPicker(tester);

        await systemBack(tester);
        expect(find.byType(BodyTypePickerPage), findsNothing);
        expect(find.byType(ProgramDetailPage), findsOneWidget);

        await systemBack(tester);
        expect(find.byType(ProgramPanel), findsNothing);
        expect(find.text('open'), findsOneWidget);
      });

      testWidgets('Esc steps from the picker to the detail, then closes',
          (tester) async {
        await pumpPanel(tester, size);
        await toPicker(tester);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.byType(BodyTypePickerPage), findsNothing);
        expect(find.byType(ProgramDetailPage), findsOneWidget);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.byType(ProgramPanel), findsNothing);
      });

      testWidgets('a tap outside closes the whole panel from the picker (N11)',
          (tester) async {
        await pumpPanel(tester, size);
        await toPicker(tester);

        await tester.tapAt(const Offset(8, 8));
        await tester.pumpAndSettle();
        expect(find.byType(ProgramPanel), findsNothing);
        expect(find.text('open'), findsOneWidget);
      });

      testWidgets('✕ closes the whole panel from the picker', (tester) async {
        await pumpPanel(tester, size);
        await toPicker(tester);

        await tester.tap(find.byKey(const Key('panel-close')));
        await tester.pumpAndSettle();
        expect(find.byType(ProgramPanel), findsNothing);
      });
    });
  }

  group('AC8 choosing completes inside the panel', () {
    testWidgets(
        'picker → proposals → choose sends starts_on and lands on the new '
        'program, the panel still open', (tester) async {
      when(() => programService.syntheticMatch(any(), any()))
          .thenAnswer((_) async => sampleSyntheticMatch());
      when(() => programService.chooseSyntheticProgram(any(), any(), any(),
              startsOn: any(named: 'startsOn')))
          .thenAnswer((_) async => sampleUserProgram());
      await pumpPanel(tester, phone);
      await toPicker(tester);

      await tester.tap(find.text(bodyShapeName(BodyShape.mesomorph, l10n)));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fatBandName(FatBand.moderate, l10n)));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text(l10n.findMyProgram));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.findMyProgram));
      await tester.pumpAndSettle();
      expect(find.byType(SyntheticProposalsPage), findsOneWidget);

      // The chosen program is what the detail reads back. Interactions are
      // cleared here, so the re-read is counted from the choose on (the
      // provider also re-reads once the restored session's user id arrives).
      when(() => programService.getCurrent())
          .thenAnswer((_) async => sampleUserProgram());
      clearInteractions(programService);
      await tester.tap(find.text('Low-Volume Mass Builder'));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.chooseThisProgram));
      await tester.pumpAndSettle();

      verify(() => programService.chooseSyntheticProgram(
            'heavy-duty-mass',
            BodyShape.mesomorph,
            FatBand.moderate,
            startsOn: dateOnly(DateTime.now()),
          )).called(1);
      expect(find.byType(SyntheticProposalsPage), findsNothing);
      expect(find.byType(BodyTypePickerPage), findsNothing);
      expect(find.byType(ProgramPanel), findsOneWidget);
      expect(find.byKey(const Key('panel-close')), findsOneWidget);
      expect(find.text('Heavy Duty Mass'), findsOneWidget);
      expect(find.byKey(const Key('change-program')), findsOneWidget);
      verify(() => programService.getCurrent()).called(1);
    });

    testWidgets('with a program, Cambiar programa opens the picker',
        (tester) async {
      when(() => programService.getCurrent())
          .thenAnswer((_) async => sampleUserProgram());
      await pumpPanel(tester, desktop);
      expect(find.text('Heavy Duty Mass'), findsOneWidget);

      await tester.tap(find.byKey(const Key('change-program')));
      await tester.pumpAndSettle();
      expect(find.byType(BodyTypePickerPage), findsOneWidget);
      expect(find.byType(ProgramPanel), findsOneWidget);
    });
  });
}
