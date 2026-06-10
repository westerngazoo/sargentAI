# R-0006 — Photo-session backend

- **Status:** Met
- **Milestone:** M2 (Logging core) — pulled forward by the differentiator fast-track
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-10
- **Depends on:** R-0002 (Done — auth, `AuthenticatedUser`, `AppState`, the `db` seam, `ApiError`)
- **Realized by:** [SPEC-0006](../specs/0006-photo-session.md) (Implemented)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

An authenticated user can **capture a progress-photo session**: create a session,
**upload one or more photos** to it (each optionally tagged with an angle), list
and view their sessions, **download a photo's image bytes**, and delete a photo
or a whole session. Image bytes are uploaded **through the API** (multipart) and
persisted to an **object store behind a storage seam**; only metadata lives in
Postgres. Every photo belongs to the uploading user and is reachable by no one
else.

This is the third M2 logging resource and the **first server-side work of the
differentiator fast-track**: it is the substrate the photo→archetype matching
(R-0013) consumes — that requirement runs pose estimation over these bytes and
derives the frame features that pick a user's archetype. R-0006 itself does **no
image processing**; it is upload, storage, retrieval, and deletion only.

## 2. Rationale

The product's differentiator starts with "upload your photo and the AI gives you
your archetype." Nothing can store a photo today. R-0006 builds that foundation
the thin-client way (bytes go through the Rust API, the intelligence stays
server-side) and introduces **object storage** to the stack behind a
dependency-inverted seam — so dev and CI run against a local filesystem store
with no cloud, and the real S3-compatible store is wired at deploy (R-0026).

## 3. Acceptance criteria

- **AC1.** A migration creates the photo tables: a **session** (`id`, `user_id`
  FK `ON DELETE CASCADE`, `performed_on` date, timestamps) and its **photos**
  (`id`, `session_id` FK `ON DELETE CASCADE`, optional `angle`, `storage_key`,
  `content_type`, `byte_size`, `created_at`). Deleting a user cascades to
  sessions and photos; deleting a session cascades to photos.
- **AC2.** `POST /photo-sessions` creates an empty session for the caller
  (`performed_on` = today) → `201` + the session (id, performed_on, empty
  photos, timestamps). `401` unauthenticated.
- **AC3.** `POST /photo-sessions/:id/photos` accepts a **multipart** image
  upload with an optional `angle` field. The API validates the content
  (**`image/jpeg`** or **`image/png`** only; size ≤ a configured cap, default
  10 MB), writes the bytes to the object store under a **non-guessable key**,
  records the metadata, and returns `201` + the photo metadata (never the
  bytes). A non-image, oversized, or malformed body → `400`. Uploading to a
  missing or foreign session → `404`.
- **AC4.** `angle`, when given, is one of a controlled set
  (`front`/`back`/`left`/`right`/`other`); an unknown angle → `400`. A photo may
  have **no angle** (flexible list — owner decision). A session may hold **any
  number** of photos.
- **AC5.** `GET /photo-sessions` → `200` + the caller's sessions (newest
  `performed_on` first), each with its photos' **metadata only**. Empty → `[]`.
  `GET /photo-sessions/:id` → `200` owned / `404` missing or foreign.
- **AC6.** `GET /photo-sessions/:id/photos/:photoId` **streams the image bytes**
  with the stored `content_type`, only to the owner; `404` if the session or
  photo is missing or owned by another user. The bytes come from the object
  store via the storage seam.
- **AC7.** `DELETE /photo-sessions/:id/photos/:photoId` removes the photo's row
  **and its stored bytes** → `204`; second delete → `404`; foreign → `404`.
  `DELETE /photo-sessions/:id` removes the session, its photo rows, and **all
  their stored bytes** → `204`; foreign/missing → `404`.
- **AC8.** **Object storage is a seam:** a storage abstraction (`put`/`get`/
  `delete` by key) with a **local filesystem implementation** used by dev and
  the test suite (no cloud). The real S3-compatible implementation is **deferred
  to the deploy requirement (R-0026)**; the seam makes that a drop-in.
- **AC9.** **Privacy & isolation:** storage keys are non-guessable and namespaced
  by user; every session/photo route is scoped to the token's `sub`; cross-user
  access returns **`404`, never `403`** (enumeration-safety, the R-0003/R-0004/
  R-0005 rule). No photo bytes are ever served to a non-owner.
- **AC10.** Errors are typed and mapped: validation → `400`, unauthenticated →
  `401`, missing/foreign → `404`, storage/DB failure → `500` (logged, no leak).
  A storage write that fails after the row would be written must not leave a
  dangling row (write bytes first, then the row; or compensate) — no orphaned
  metadata pointing at absent bytes.
- **AC11.** **Tests:** unit tests for the domain (angle parsing, content-type/
  size validation) and the local object store; `#[sqlx::test]` integration
  tests for every endpoint incl. the multipart upload, the byte download
  round-trip, cross-user `404`, and cascade deletes removing stored bytes. All
  gates green: `cargo fmt`/`clippy`/`test`/`build`.
- **AC12.** **No image processing** (pose estimation, resizing, EXIF stripping,
  thumbnails) — that is R-0013/R-0018 (M6). No mobile UI (the capture screen is
  a separate M3 requirement). Backend only.

## 4. Constraints & non-goals

- **No pose estimation / CV / derived features** — R-0013/R-0018.
- **No real S3 wiring in this requirement** — the seam + local impl only; S3 at
  R-0026.
- **No presigned-URL direct-to-S3 upload** — bytes go through the API (owner
  decision); presigned is a possible later scale optimization.
- **No image transforms** — no resize, re-encode, thumbnail, EXIF scrub, or
  format conversion; bytes are stored and served verbatim.
- **No mobile capture UI** — separate M3 requirement (re-homed photo-capture
  screen), gated on this.
- **No per-day uniqueness / fixed four-slot model** — a flexible photo list per
  session (owner decision); angle is optional metadata.
- **No CDN / signed download URLs / public sharing** — owner-only authenticated
  download.

## 5. Open questions

Settled in the step-1 discussion (owner, 2026-06-10):

- **OQ1 — Upload mechanism?** RESOLVED → **API-proxy multipart** (bytes through
  the API); presigned deferred. (AC3)
- **OQ2 — Session/angle model?** RESOLVED → **flexible photo list**, angle
  optional from a controlled set. (AC4)
- **OQ3 — Storage backend now?** RESOLVED → **storage seam + local filesystem
  impl** for dev/CI; real S3 deferred to R-0026. (AC8)

Deferred to the SPEC-0006 design discussion (HOW): the exact storage-key scheme,
the multipart-size enforcement point (stream vs buffer), the
`ObjectStore` trait surface, whether session create + first photo can be one
call, and the write-order/compensation detail for AC10.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **API-proxy multipart upload; bytes through the Rust API.** | Simplest to build/test; the API already needs the bytes for the server-side pose estimation R-0013 runs. Presigned is a later scale optimization. (OQ1) |
| 2026-06-10 | **Flexible photo list per session; optional angle from a controlled set.** | Owner decision; less rigid than fixed four slots, angle stays useful metadata for the fixed-angle pose features. (OQ2) |
| 2026-06-10 | **Object storage behind a `put/get/delete` seam; local filesystem impl now, S3 at R-0026.** | Dependency-inverted storage keeps dev/CI cloud-free and testable; the real impl is a drop-in at deploy. (OQ3) |
| 2026-06-10 | **Metadata in Postgres, bytes in the object store; never bytes in the DB.** | Standard separation; keeps the DB small and the bytes streamable. |
| 2026-06-10 | **Cross-user access → `404`, never `403`; non-guessable, user-namespaced keys.** | Enumeration-safety + health-data privacy (the domain notes flag photos as sensitive). |
| 2026-06-10 | **No image processing in R-0006.** | Storage/retrieval only; CV is M6 (R-0013/R-0018). |

## Changelog

- _2026-06-10 — created (Draft). Fast-track: the photo-session backend, substrate for photo→archetype matching (R-0013). Three step-1 forks owner-resolved (API-proxy multipart; flexible photo list; storage seam + local impl)._
- _2026-06-10 — **Accepted.** Owner accepted AC1–AC12. Next: step 2 — SPEC-0006 and the architect design review._
