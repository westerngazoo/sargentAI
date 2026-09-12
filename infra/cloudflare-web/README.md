# Goose Physics web — Cloudflare Workers static assets

Hosts the Flutter web build. Free-plan compatible, unlike the container-backed
API in [`../cloudflare`](../cloudflare), which needs Workers Paid.

Live: <https://sargent-ai.gustavo-delgadillo.workers.dev>

## The one thing that will bite you

`API_BASE_URL` is **compile-time**, not runtime —
[`AppConfig.apiBaseUrl`](../../mobile/lib/src/core/config/app_config.dart) reads
it via `String.fromEnvironment`, defaulting to `http://localhost:8080`. A build
made without the flag ships that default, and on a phone `localhost` means *the
phone itself*, so the app silently cannot reach any API. Always build with the
flag:

```bash
cd mobile
flutter build web --release \
  --dart-define=API_BASE_URL=https://sargent-api.goosethropic.systems
```

Verify before deploying (the value must appear, and `localhost:8080` must not):

```bash
grep -c "sargent-api.goosethropic.systems" build/web/main.dart.js   # want >= 1
grep -c "localhost:8080"                   build/web/main.dart.js   # want 0
```

Checking the *deployed* copy needs `curl --compressed`; without it you are
grepping compressed bytes and will always get 0.

## Deploy

```bash
cd infra/cloudflare-web
npx wrangler deploy
```

`not_found_handling: single-page-application` is required — `go_router` owns
client-side routing, so unknown paths must serve `index.html` rather than 404.

### `.assetsignore`

Workers rejects any asset over 25 MiB, and `mobile/build/web/` may contain a
~170 MB `sargent-ai.apk` (the Android build, served for sideloading). Exclude it
with `mobile/build/web/.assetsignore`:

```
sargent-ai.apk
```

The APK therefore is **not** distributed by this Worker; serve it separately
(R2 or the tunnel) if sideloading is needed.

## Installing on a phone

No APK required — the manifest sets `name: Goose Physics` and
`display: standalone`, so **Add to Home Screen** installs it as an app icon.
Building the APK instead needs a JDK (`assembleRelease` fails with "Unable to
locate a Java Runtime" otherwise).

## Related

- [`../cloudflare`](../cloudflare) — the API on Containers (needs Workers Paid).
- [`../../backend/fly.toml`](../../backend/fly.toml) — the Fly.io alternative
  for the API.
