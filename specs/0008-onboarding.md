# SPEC-0008 — Onboarding flow

- **Status:** Accepted
- **Realizes:** R-0008
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-04
- **Depends on:** SPEC-0007 (Implemented) — reuses the `lib/src/<feature>/{domain,data,application,presentation}` layering, the Riverpod `ProviderScope`, the shared `dioProvider` (Bearer attach + 401 sink), `ApiException`, `AppConfig`, the `go_router` auth gate, and the `AuthForm`/screen idioms; SPEC-0003 (Implemented) — the `GET`/`PUT /profile/me` contract.
- **Module(s):** `mobile/lib/src/profile/**` (new feature module), `mobile/lib/src/shell/home_shell.dart` (prompt integration), `mobile/lib/src/router/app_router.dart` (route), `mobile/test/profile/**` + `mobile/test/shell/**` (tests).

## 1. Motivation

Realizes [R-0008](../requirements/0008-onboarding.md): give a signed-in user the
first in-app way to record their **body stats and goals**, so the M4 archetype
prior and M5 ML engine have the inputs R-0003 can already store. Onboarding is an
**optional, dismissible prompt** from the home shell (gated on `GET /profile/me`
→ `404`) that opens a **multi-step wizard** writing the profile via
`PUT /profile/me`. No backend changes; it is pure thin-client work on the R-0007
foundation.

## 2. Design

### 2.1 Shape

A new `profile` feature module mirrors the R-0007 `auth` module's four layers.
The home shell observes profile existence through one async provider and renders
a dismissible prompt; the wizard owns a draft and submits it.

```
lib/src/profile/
  domain/
    goal.dart            # Goal enum  <-> wire snake_case
    sex.dart             # Sex enum   <-> wire snake_case (R-0003 set)
    profile.dart         # Profile (parsed GET /profile/me response)
    profile_draft.dart   # wizard draft + field validation (mirrors backend); toRequest()
  data/
    profile_api.dart     # getMe() -> Profile?  (404 -> null); putMe(req) -> Profile
    profile_repository.dart  + profileRepositoryProvider
  application/
    profile_providers.dart   # profileProvider (FutureProvider<Profile?>) ; onboardingDismissedProvider
    onboarding_controller.dart # OnboardingController (Notifier): step, draft, busy, error; submit()
  presentation/
    profile_prompt.dart      # dismissible "complete your profile" banner
    onboarding_screen.dart   # wizard scaffold: progress + back/next, hosts the steps
    steps/body_stats_step.dart
    steps/goals_step.dart
    steps/optional_details_step.dart
```

### 2.2 Profile existence & the prompt (AC1/AC2)

`profileProvider` is a `FutureProvider<Profile?>` that calls
`ProfileRepository.getMe()`, which maps a `404` to `null` (no profile) and a
`200` to a parsed `Profile`. The **home shell watches it and renders via
`AsyncValue.when`** — adopting the uniform async idiom the architect flagged for
R-0007's hand-rolled `HomeShell._load` (SPEC-0007 review, Finding 1); this spec
**also refactors `HomeShell` to drive its `GET /auth/me` through the same
`AsyncValue` pattern** so the shell is a clean template for R-0009+.

- `data == null` → show `ProfilePrompt` (a dismissible banner above the shell
  body) **unless** `onboardingDismissedProvider` is `true` for this session.
- `data != null` → no prompt; the shell shows the profile summary.
- `loading` → a spinner; `error` (non-404) → a retryable message.

`onboardingDismissedProvider` is a session-scoped `StateProvider<bool>` (not
persisted — AC2 is "for the session"). Dismissing sets it `true`.

**Cold-start liveness (preserves SPEC-0007 SAC5).** The refactored shell drops
the old `HomeShell` `GET /auth/me` call (it fetched only the user id, which is
already in the in-memory session). `profileProvider`'s `GET /profile/me` becomes
the new liveness probe: a restored-but-expired token yields a `401` there, which
the shared `AuthInterceptor` 401-sinks → logout → router redirect to `/login`
(R-0007 AC5/SAC5) — so the stale-token path is **kept**, not regressed (architect
note on OQ-E3). A `404` (valid token, no profile) correctly shows the prompt; a
`200` shows the profile. R-0009+ inherit this: the shell already probes liveness,
so feature screens must **not** reintroduce a redundant `/auth/me` read.

### 2.3 The wizard (AC3–AC7)

`OnboardingController extends Notifier<OnboardingState>` where `OnboardingState`
holds `{ int step, ProfileDraft draft, bool submitting, String? error }`. The
screen is a `PageView`/`Stepper`-style scaffold with a progress indicator and
back/next controls; **draft data lives in the controller, so it survives step
navigation** (AC3). Steps:

1. **Body stats** — `date_of_birth` (a date picker), `height_cm` (int),
   `weight_kg` (double). Required. **Next is disabled** until all three pass the
   mirrored validators (AC4).
2. **Goals** — multi-select chips over the five `Goal`s; **≥1 required** to
   finish (AC5).
3. **Optional details** — `sex` (choice, clearable) and `body_fat_percentage`
   (double, [1.0, 75.0]); both **skippable** (AC6).

`ProfileDraft` is immutable with `copyWith`; it exposes per-field validators that
return `null` (ok) or a message, mirroring the backend exactly:
age ∈ [13, 120] (derived from `date_of_birth` vs today), height ∈ [50, 300],
weight ∈ [20.0, 500.0], body-fat ∈ [1.0, 75.0]. `draft.toRequest()` builds the
`ProfileRequest` (omitting empty optionals) and is only callable once the
required fields validate.

### 2.4 Submit, success, and failure (AC7/AC8)

On the final step the `OnboardingScreen`'s finish button calls
`controller.submit()`. `submit()` owns the network call and records the outcome
**on `OnboardingState`** — never by throwing to the widget (architect finding 3);
the screen `ref.listen`s the state and reacts:

- **`200`** → `submit()` `await ref.refresh(profileProvider.future)` so the new
  profile is cached *before* navigation, then sets `state.done = true`; the
  screen's `ref.listen` sees `done` and calls `context.go('/home')`. The shell,
  re-reading a non-null profile, shows no prompt (AC7). The `OnboardingScreen` is
  the sole owner of the `go('/home')` (architect finding 5).
- **`400 {field}`** → `submit()` sets `state.error` (message) and
  `state.errorStep = stepFor(field)` (`date_of_birth`/`height_cm`/`weight_kg` →
  body-stats; `body_fat_percentage` → optional; else final step); the screen
  `ref.listen`s `errorStep`, jumps there, and shows the message. Draft untouched
  (AC8); the user fixes and resubmits.
- **transport/timeout (`statusCode == null`)** → `state.error` = retryable
  message; draft intact (AC8).
- **`401`** → the shared `AuthInterceptor` already clears the session and the
  router redirects to `/login` (R-0007 AC5); `submit()` does nothing special.

No double-submit: the finish button is disabled while `state.submitting` (the
R-0007 `AuthForm` idiom). The widget holds no `try/catch` — failure is **data on
the state**, not control flow (AC9/SAC9).

### 2.5 Routing (AC3/AC7)

`app_router.dart` gains an authenticated `/onboarding` route (under the same
auth-gate — only reachable when `AuthAuthenticated`). The prompt navigates
`context.go('/onboarding')`; a wizard "close/skip" returns to `/home`. The
profile-existence check does **not** gate routing (onboarding is optional, OQ1) —
it only drives the prompt's visibility.

### 2.6 Data types & wire mapping

- `Goal` ⇄ `lose_fat` / `build_muscle` / `recomp` / `maintain` / `gain_strength`
  (`#[serde(rename_all = "snake_case")]`).
- `Sex` ⇄ **`male` / `female`** — `core::profile::Sex` is
  `#[serde(rename_all = "lowercase")]`, **not** snake_case (architect finding 2,
  confirmed against `backend/crates/core/src/profile.rs`). The client enum must
  emit exactly these tokens (AC11: no backend change).
- `date_of_birth` ⇄ `YYYY-MM-DD` (ISO `NaiveDate`).
- `ProfileRequest` JSON keys exactly match the backend DTO; skipped optionals are
  **omitted** (the backend `#[serde(default)]`s them).
- `Profile` parses the `ProfileResponse` keys; `age` is read from the response
  (server-derived), never recomputed on device.

### 2.7 Error parsing — shared, flat-body (corrects an R-0007 latent bug)

AC8 needs the offending **field** from a `400`. The backend error body is **flat**
— `{"error": "<kind>", "field": "<name>"}` where `error` is a *string* kind and
`field` is present only for `validation` (`backend/crates/api/src/error.rs`). But
the R-0007 `AuthApi._toApiException` only reads `field`/`message` when
`data['error'] is Map` (a **nested** shape the backend never sends), so
`ApiException.field` is **always `null`** today — latent because the auth screens
map no fields and fall through to a per-status default (architect finding 1).

This spec **hoists a corrected, shared constructor into `core/network`** and has
**both** `auth` and `profile` use it:

```dart
// core/network/api_exception.dart
factory ApiException.fromDio(DioException e) {
  final res = e.response;
  if (res == null) return const ApiException("can't reach the server — retry"); // transport/timeout
  final data = res.data;
  final field = (data is Map) ? data['field'] as String? : null;  // FLAT body
  return ApiException(_defaultMessage(res.statusCode), statusCode: res.statusCode, field: field);
}
```

`AuthApi._toApiException` is replaced by `ApiException.fromDio` (no auth behaviour
change — auth still ignores `field` and uses the per-status default message);
`ProfileApi` uses the same constructor so `field` is populated for AC8/OQ-E5.
This is the single error-parsing authority every future logger (R-0009+) reuses,
so the broken nested-parse is not copied forward.

## 3. Code outline

Representative snippets (final form reconciled in step 5 against analyzer +
`dart format`). Tests are authored by `qa` in step 3 against §6.

### 3.1 `domain/profile_draft.dart`

```dart
@immutable
class ProfileDraft {
  const ProfileDraft({
    this.dateOfBirth, this.heightCm, this.weightKg,
    this.goals = const {}, this.sex, this.bodyFatPercentage,
  });

  final DateTime? dateOfBirth;
  final int? heightCm;
  final double? weightKg;
  final Set<Goal> goals;
  final Sex? sex;
  final double? bodyFatPercentage;

  ProfileDraft copyWith({ /* nullable overrides + clear flags */ });

  // Validators mirror the backend (return null when ok).
  String? dobError(DateTime today);   // age in [13,120]
  String? heightError();              // [50,300]
  String? weightError();              // [20.0,500.0]
  String? bodyFatError();             // [1.0,75.0] when present
  bool get bodyStatsValid;            // all three required fields ok
  bool get goalsValid;                // goals.isNotEmpty

  /// Total — returns the request only when the required fields validate, else
  /// `null` (no precondition throw — architect finding 4); `submit()` guards on
  /// non-null before `putMe`.
  ProfileRequest? toRequest();
}
```

### 3.2 `data/profile_api.dart`

```dart
class ProfileApi {
  ProfileApi(this._dio);
  final Dio _dio;

  /// GET /profile/me — 404 means "no profile yet" (-> null), not an error.
  Future<Profile?> getMe() async {
    try {
      final res = await _dio.get<Map<String, dynamic>>('/profile/me');
      return Profile.fromJson(res.data!);
    } on DioException catch (e) {
      if (e.response?.statusCode == 404) return null;
      throw ApiException.fromDio(e);
    }
  }

  Future<Profile> putMe(ProfileRequest req) async {
    try {
      final res = await _dio.put<Map<String, dynamic>>('/profile/me', data: req.toJson());
      return Profile.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e); // carries statusCode + field for AC8
    }
  }
}
```

### 3.3 `application/profile_providers.dart`

```dart
final profileApiProvider = Provider((ref) => ProfileApi(ref.read(dioProvider)));
final profileRepositoryProvider = Provider((ref) => ProfileRepository(ref.read(profileApiProvider)));

/// Drives the home prompt (null => no profile => offer onboarding).
final profileProvider = FutureProvider<Profile?>(
  (ref) => ref.read(profileRepositoryProvider).getMe(),
);

/// Session-only dismissal of the prompt (not persisted — AC2).
final onboardingDismissedProvider = StateProvider<bool>((_) => false);
```

### 3.4 `application/onboarding_controller.dart`

```dart
class OnboardingState {
  const OnboardingState({this.step = 0, this.draft = const ProfileDraft(),
    this.submitting = false, this.error, this.errorStep, this.done = false});
  final int step; final ProfileDraft draft; final bool submitting;
  final String? error;     // inline message for the current/target step
  final int? errorStep;    // step the screen should jump to on a 400 (finding 5)
  final bool done;         // success signal the screen ref.listens to navigate
  OnboardingState copyWith({...});
}

final onboardingControllerProvider =
    NotifierProvider<OnboardingController, OnboardingState>(OnboardingController.new);

class OnboardingController extends Notifier<OnboardingState> {
  @override OnboardingState build() => const OnboardingState();

  void setBodyStats({DateTime? dob, int? height, double? weight}) { /* copyWith draft */ }
  void toggleGoal(Goal g) { /* add/remove in draft.goals */ }
  void setOptional({Sex? sex, double? bodyFat, bool clearSex = false}) { /* copyWith */ }
  void next() / void back();   // bounded by step count

  Future<void> submit() async {
    final req = state.draft.toRequest();
    if (req == null) return;                       // guard: never submit invalid
    state = state.copyWith(submitting: true, error: null, errorStep: null);
    try {
      await ref.read(profileRepositoryProvider).putMe(req);
      await ref.refresh(profileProvider.future);   // profile cached before nav (AC7)
      state = state.copyWith(submitting: false, done: true); // screen ref.listens -> go('/home')
    } on ApiException catch (e) {
      // Failure is DATA on the state, not a thrown exception (finding 3):
      state = state.copyWith(
        submitting: false, error: e.message, errorStep: _stepFor(e.field));
    }
  }

  /// Map a backend 400 `field` to the step that owns it (finding 5).
  int? _stepFor(String? field) => switch (field) {
        'date_of_birth' || 'height_cm' || 'weight_kg' => 0,
        'body_fat_percentage' => 2,
        _ => null, // unmapped -> show on the final step
      };
}
```

### 3.5 `shell/home_shell.dart` (refactor to `AsyncValue` + prompt)

```dart
@override
Widget build(BuildContext context, WidgetRef ref) {
  final profile = ref.watch(profileProvider);
  final dismissed = ref.watch(onboardingDismissedProvider);
  return Scaffold(
    appBar: AppBar(title: const Text('fitAI'), actions: [ _logout(ref) ]),
    body: profile.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (e, _) => RetryableError(onRetry: () => ref.invalidate(profileProvider)),
      data: (p) => Column(children: [
        if (p == null && !dismissed) ProfilePrompt(
          onStart: () => context.go('/onboarding'),
          onDismiss: () => ref.read(onboardingDismissedProvider.notifier).state = true,
        ),
        Expanded(child: Center(child: Text(p == null
            ? 'Welcome — complete your profile to get started'
            : 'Signed in · ${p.goals.length} goal(s)'))),
      ]),
    ),
  );
}
```

> `HomeShell` becomes a `ConsumerWidget` (no more manual `_userId`/`_loading`
> state), resolving SPEC-0007 review Finding 1; the user identity, if still
> shown, reads from the existing `authControllerProvider` session.

### 3.6 `router/app_router.dart`

```dart
GoRoute(path: '/onboarding', builder: (_, __) => const OnboardingScreen()),
// reachable only under AuthAuthenticated (existing redirect gate unchanged).
```

## 4. Non-goals

Inherits R-0008 §4: no training-history field, no profile-edit/settings screen,
no blocking onboarding, no new profile fields, metric-only, **no backend
changes**, no workout/nutrition/photo entry. Also: no persistence of the
"dismissed" flag across launches (session-only); no analytics.

## 5. Open questions

**Resolved by the architect review (2026-06-04, REQUEST CHANGES → applied).**
OQ-E1/E2/E3/E4 approved as proposed; OQ-E5 approved in intent and unblocked by the
§2.7 flat-body parser fix. Three blocking/major/minor findings folded into §2.6,
§2.7, §2.2, §2.4, §3.1, §3.4 above. Details below for the record.

- **OQ-E1 — Wizard host widget: `Stepper` vs `PageView` + custom progress?**
  Proposed: `PageView` with a thin progress header and explicit back/next, for
  full control of validation gating per step.
- **OQ-E2 — `profileProvider` as `FutureProvider` vs `AsyncNotifier`?** Proposed:
  `FutureProvider<Profile?>` (read-only existence check; mutations happen via the
  controller, which `invalidate`s it). Revisit if the profile gains in-place edits.
- **OQ-E3 — `HomeShell` refactor to `AsyncValue` in this spec?** Proposed: yes —
  it's the right moment (the prompt needs an async read anyway) and it pays down
  the R-0007 Finding-1 nit before R-0009 copies the shell.
- **OQ-E4 — Where does the "dismissed" flag live?** Proposed: a session
  `StateProvider<bool>` (not persisted), matching AC2's "for the session".
- **OQ-E5 — 400-field → step mapping.** Proposed: the screen maps
  `e.field` (`date_of_birth`/`height_cm`/`weight_kg` → step 1;
  `body_fat_percentage` → step 3) and jumps to that step with the message; an
  unmapped field shows a generic inline error on the final step.

## 6. Acceptance criteria

Each maps 1:1 to an R-0008 criterion and to the qa agent's test.

- [ ] **SAC1 → AC1.** With `GET /profile/me` mocked `404`, `HomeShell` shows
  `ProfilePrompt`; mocked `200`, it does not. (widget)
- [ ] **SAC2 → AC2.** Dismissing the prompt hides it for the session
  (`onboardingDismissedProvider` true) and leaves the shell usable; nothing is
  saved. (widget)
- [ ] **SAC3 → AC3.** The wizard renders steps with progress + back/next; data
  entered in step 1 is still present after navigating to step 2 and back.
- [ ] **SAC4 → AC4.** Body-stats validation mirrors the backend: out-of-range age
  (<13 / >120), height (<50 / >300), weight (<20 / >500), and empty values block
  Next with an inline message; in-range values allow Next.
- [ ] **SAC5 → AC5.** Finish is disabled with zero goals and enabled once ≥1 of
  the five goals is selected.
- [ ] **SAC6 → AC6.** Sex and body-fat are skippable (empty → allowed); a present
  body-fat outside [1.0, 75.0] is rejected.
- [ ] **SAC7 → AC7.** Completing the wizard issues `PUT /profile/me` with exactly
  the collected fields (skipped optionals omitted); on `200` the app returns to
  home and the prompt is gone (profile re-read).
- [ ] **SAC8 → AC8.** A `400 {field}` keeps the user in the wizard with data
  intact and shows the field message; a transport error shows a retryable
  message; a `401` routes to login. No data loss in any case.
- [ ] **SAC9 → AC9.** Wizard state is a Riverpod `Notifier`; the save uses the
  shared `dioProvider`; no business logic beyond presentation/orchestration.
- [ ] **SAC10 → AC10.** `flutter analyze`, `dart format --set-exit-if-changed .`,
  and `flutter test` (the `test/` suite) are green; widget tests cover the prompt,
  each step's validation, and the save flow against a mocked `Dio`.
- [ ] **SAC11 → AC11.** No files under `backend/` change; only `GET`/`PUT
  /profile/me` are called.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-04 | **New `lib/src/profile` feature module, four layers mirroring `auth`.** | Keeps the R-0007 layering; onboarding is the profile feature's first surface. |
| 2026-06-04 | **`profileProvider` (`FutureProvider<Profile?>`) drives the prompt; `404` → `null`.** | One async source of truth for "has a profile?"; reuses R-0003's absence signal. (OQ-E2) |
| 2026-06-04 | **Refactor `HomeShell` to `AsyncValue.when` in this spec.** | Pays down SPEC-0007 review Finding 1 before R-0009 copies the shell; the prompt needs the async read regardless. (OQ-E3) |
| 2026-06-04 | **Wizard state in an `OnboardingController` `Notifier`; immutable `ProfileDraft` with `copyWith`.** | Draft survives step navigation; validation is pure and testable. (OQ-E1) |
| 2026-06-04 | **Client validators mirror the backend ranges exactly; backend stays source of truth.** | Fail fast in the UI; `PUT` still rejects bad data (AC8). |
| 2026-06-04 | **Session-only `onboardingDismissedProvider`; not persisted.** | AC2 is "for the session"; persistence is a later concern. (OQ-E4) |
| 2026-06-04 | **No backend changes.** | R-0003's upsert + absence signal suffice; M3 is thin-client. |
| 2026-06-04 | **(architect) Hoist a shared, flat-body `ApiException.fromDio` into `core/network`; auth + profile use it.** | The R-0007 parser read a nested `{"error":{...}}` shape the backend never sends, so `field` was always null — latent until AC8 needs it (finding 1). One corrected authority all loggers reuse. |
| 2026-06-04 | **(architect) `Sex` wire tokens are `male`/`female` (lowercase), not snake_case.** | `core::profile::Sex` is `rename_all = "lowercase"`; the client must match (AC11). (finding 2) |
| 2026-06-04 | **(architect) Submit failure is data on `OnboardingState` (`error`/`errorStep`/`done`), not a thrown exception; the screen `ref.listen`s.** | One failure channel; keeps widgets free of `try/catch`/business logic (AC9). (finding 3) |
| 2026-06-04 | **(architect) `toRequest()` is total (`ProfileRequest?`), no precondition throw; `submit()` guards on non-null.** | Removes an unchecked-failure surface (CLAUDE.md §6). (finding 4) |
| 2026-06-04 | **(architect) `profileProvider`'s `GET /profile/me` becomes the cold-start liveness probe; the old `/auth/me` read is dropped.** | Preserves SPEC-0007 SAC5 (a 401 there 401-sinks → login) without a redundant call; R-0009+ must not re-add `/auth/me`. (OQ-E3 note) |

## Changelog

- _2026-06-04 — created (Draft). Realizes the accepted R-0008. Five HOW-level design questions (OQ-E1..E5) raised for the architect review; proposes paying down the SPEC-0007 `HomeShell` `AsyncValue` nit here._
- _2026-06-04 — **Accepted.** Architect review returned REQUEST CHANGES with one blocking finding (the nested-vs-flat error-body parser, dead since R-0007 and load-bearing for AC8), one major (`Sex` = `male`/`female`), and three minor (state-not-throw failure channel; total `toRequest()`; named submit-success navigation owner), plus the OQ-E3 liveness caveat. All applied in lockstep: §2.6 (Sex), §2.7 (shared flat-body `ApiException.fromDio`, fixing the R-0007 latent bug), §2.2 (liveness preserved), §2.4 + §3.4 (failure-as-state, no rethrow, refresh-before-nav), §3.1 (total `toRequest`). OQ-E1/E2/E4 approved as proposed; OQ-E5 unblocked by §2.7._
