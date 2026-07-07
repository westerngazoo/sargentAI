# fitai-api on Cloudflare Containers — SPIKE

Deploys the **existing** `backend/Dockerfile` (Rust + Axum + native ONNX Runtime)
to [Cloudflare Containers](https://developers.cloudflare.com/containers/) behind a
thin Worker. Nothing in `backend/` changes — the container runs the same image we
build for CI, so the ONNX pose pipeline works exactly as it does locally.

> **This is a spike, not production.** It proves the image runs on Cloudflare and
> that requests route through. Read the caveats before relying on it.

## Layout

```
infra/cloudflare/
├── wrangler.jsonc     # container binding → ../../backend (build context)
├── src/index.ts       # Worker: proxy request → container :8080
├── package.json       # @cloudflare/containers + wrangler
├── tsconfig.json
└── .dev.vars.example  # local secrets template
```

The Worker is deliberately thin: it forwards every request to the container's
port 8080 and returns the response. All logic stays in the Rust image.

## Prerequisites

- Docker running locally (wrangler builds/runs the image for `dev`).
- A reachable **Postgres** with the migrations applied (the app does not create
  its own DB). For local dev, `host.docker.internal` reaches your host Postgres.
- Node + `npm install` in this directory.

## Local dev

```bash
cd infra/cloudflare
npm install
cp .dev.vars.example .dev.vars   # fill in DATABASE_URL, JWT_SECRET, ...
npm run dev                      # wrangler builds the image and runs it via Docker
# → http://localhost:8787/health  should proxy to the container's /health
```

## Deploy

```bash
wrangler secret put DATABASE_URL
wrangler secret put JWT_SECRET
wrangler secret put ANTHROPIC_API_KEY      # optional (voice-intent LLM)
wrangler secret put FDC_API_KEY            # optional (USDA lookup)
wrangler secret put GOOGLE_OAUTH_AUDIENCE  # optional (Google sign-in)
npm run deploy
```

## Instance sizing

`standard-2` (1 vCPU / 6 GiB) in `wrangler.jsonc` — comfortable for ONNX Runtime
+ Axum. Bump to `standard-3/4` or a `instance_type_custom` block if pose
inference gets heavy; GPUs are now available on the platform if it ever needs
them.

## Caveats (why this is a spike, not prod)

- **Database:** the container connects to an external Postgres over the internet.
  For production, front it with **[Hyperdrive](https://developers.cloudflare.com/hyperdrive/)**
  (connection pooling + edge caching) instead of a raw connection string.
- **Photo storage:** `PHOTO_STORE_ROOT=/tmp/photos` is on the container's
  **ephemeral disk** and is wiped on every stop. Production must write photos to
  **[R2](https://developers.cloudflare.com/r2/)**, not local disk.
- **Cold starts:** ~2–3s when an instance spins up (`sleepAfter = "15m"` scales
  to zero when idle).
- **No autoscaling:** instances don't scale automatically. This spike uses one
  named instance; switch `getContainer(...)` to `getRandom(env.FITAI_API, N)` in
  `src/index.ts` to spread load across `N` instances.
- **Regions:** Containers run in a limited set of regions today.

## Not wired into the requirement loop

This is exploratory. If we adopt it, it becomes a proper requirement/spec (deploy
target, Hyperdrive, R2 for photos, secrets management, CI deploy step).
