// R-0035 AC-level regression: the silent-audio loop that keeps the earbud
// foreground service alive.
//
// `AudioServiceHandler._initAudio` calls
// `_player.setAsset('assets/audio/silence.mp3')` and swallows any failure:
//
//     } catch (_) {
//       // Silently degrade on load failure (AC11)
//     }
//
// That catch is deliberate and correct — a dead audio file must not crash a
// session. Its cost is that a MISSING asset is indistinguishable from a working
// one at runtime: `startSilentLoop()` plays nothing, the foreground service is
// never held open, and the media button stops advancing the session once the
// phone is pocketed. The feature reports success and does nothing.
//
// The file has always existed on disk (mobile/assets/audio/silence.mp3, 904
// bytes) but was never declared under `flutter:` in pubspec.yaml, so it was
// never bundled. Nothing failed loudly enough to notice.
//
// This test asserts the contract the catch block hides: the asset the handler
// asks for is actually in the bundle.

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const assetPath = 'assets/audio/silence.mp3';

  setUpAll(TestWidgetsFlutterBinding.ensureInitialized);

  test('the silent-audio asset is bundled, not merely present on disk',
      () async {
    // rootBundle resolves only what pubspec.yaml declares. An undeclared file
    // sitting in the source tree throws here, which is exactly the production
    // failure — except that in production the throw is swallowed.
    final bytes = await rootBundle.load(assetPath);
    expect(bytes.lengthInBytes, greaterThan(0),
        reason: '$assetPath must be a real, non-empty audio file');
  });

  test('the bundled asset is a plausible MP3, not a placeholder', () async {
    final bytes = await rootBundle.load(assetPath);
    final head = bytes.buffer.asUint8List(0, 3);

    // Either an ID3v2 tag ("ID3") or an MPEG frame sync (0xFF 0xEx/0xFx).
    final isId3 = head[0] == 0x49 && head[1] == 0x44 && head[2] == 0x33;
    final isFrameSync = head[0] == 0xFF && (head[1] & 0xE0) == 0xE0;

    expect(isId3 || isFrameSync, isTrue,
        reason: 'just_audio cannot loop a file that is not decodable audio; '
            'got leading bytes ${head.map((b) => b.toRadixString(16)).toList()}');
  });
}
