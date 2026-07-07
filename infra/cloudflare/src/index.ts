// SPIKE — thin Worker in front of the fitai-api container.
//
// The Worker's only job is to hand the request to the container and proxy the
// response. All application logic stays in the Rust image unchanged; ONNX
// Runtime is already baked into that image, so pose estimation "just works"
// the same way it does locally.

import { Container, getContainer } from "@cloudflare/containers";

export interface Env {
  FITAI_API: DurableObjectNamespace<FitaiApi>;
  // Secrets — set with `wrangler secret put <NAME>` (see README).
  DATABASE_URL: string;
  JWT_SECRET: string;
  ANTHROPIC_API_KEY?: string;
  FDC_API_KEY?: string;
  GOOGLE_OAUTH_AUDIENCE?: string;
}

export class FitaiApi extends Container<Env> {
  // Axum binds 0.0.0.0:$PORT (default 8080); EXPOSE 8080 in the Dockerfile.
  defaultPort = 8080;
  // Readiness gate — the container is "up" once /health answers.
  pingEndpoint = "/health";
  // Scale to zero after inactivity to keep the spike cheap.
  sleepAfter = "15m";

  // Forwarded into the container process; the Rust app reads these via env::var.
  // Sourced from Worker secrets so nothing sensitive lives in config.
  envVars = {
    PORT: "8080",
    // Ephemeral container disk — fine for a spike; move to R2 for production.
    PHOTO_STORE_ROOT: "/tmp/photos",
    DATABASE_URL: this.env.DATABASE_URL,
    JWT_SECRET: this.env.JWT_SECRET,
    ANTHROPIC_API_KEY: this.env.ANTHROPIC_API_KEY ?? "",
    FDC_API_KEY: this.env.FDC_API_KEY ?? "",
    GOOGLE_OAUTH_AUDIENCE: this.env.GOOGLE_OAUTH_AUDIENCE ?? "",
  };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // Stateless HTTP API — all real state lives in Postgres, so any instance
    // can serve any request. A single named instance keeps the spike simple;
    // swap for `getRandom(env.FITAI_API, N)` to spread load across N instances.
    const container = getContainer(env.FITAI_API, "fitai-api");
    return container.fetch(request);
  },
};
