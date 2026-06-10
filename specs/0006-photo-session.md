# SPEC-0006 — Photo-session backend

- **Status:** Accepted
- **Realizes:** R-0006
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-10
- **Depends on:** SPEC-0002 (Implemented) — `AppState`, `AuthenticatedUser`, the `db` seam + `into_*` corruption discipline, `ApiError`, the migration/CI/dev-DB machinery; SPEC-0004/0005 (Implemented) — the per-user owner-scoped CRUD shape and enumeration-safety rule.
- **Module(s):** `backend/crates/core/photo` (new), `backend/crates/api/storage` (new — the `ObjectStore` seam), `backend/crates/api/photo` (new — HTTP), `backend/crates/api/{db,lib,main}` (extended), `backend/migrations/` (new file).

## 1. Motivation

Realizes [R-0006](../requirements/0006-photo-session.md): the photo-session
backend — session CRUD plus **multipart photo upload through the API** to an
**object-store seam**, metadata in Postgres. It is the substrate the
photo→archetype matching (R-0013) consumes. Two novelties over R-0004/R-0005:
**binary bytes** (uploaded multipart, streamed back) and an **infrastructure
seam** (`ObjectStore`) so the bytes live outside Postgres and outside the cloud
during dev/CI.

## 2. Design

### 2.1 Shape

```
users (R-0002)
  └── photo_sessions (id, user_id→users, performed_on, created_at, updated_at)
        └── photo_session_photos (id, session_id→photo_sessions, angle?,
                                  storage_key, content_type, byte_size, created_at)
object store (seam):  key = "{user_id}/{session_id}/{photo_id}"  →  raw bytes
```

Metadata is in Postgres; **bytes never touch the DB**. The `storage_key` is the
only link, and it is UUID-only (non-guessable) and user-namespaced (AC9).

### 2.2 Layering

- **`core::photo`** (pure — no axum/sqlx/storage/IO):
  - `Angle` enum (`Front`/`Back`/`Left`/`Right`/`Other`) ⇄ snake/lowercase wire,
    with `parse` (unknown → error);
  - `ImageContentType` (`ImageJpeg`/`ImagePng`) ⇄ `image/jpeg`/`image/png`, with
    `parse` (anything else → error) — the AC3 allowlist;
  - `NewPhoto` write model: validates an `Option<Angle>` + `ImageContentType` +
    `byte_size` (`1..=MAX_BYTES`, `MAX_BYTES = 10 * 1024 * 1024`) → built via
    `::new`, returning `PhotoError`;
  - read aggregates `PhotoSession { id, user_id, performed_on, photos, ts }` and
    `SessionPhoto { id, angle?, content_type, byte_size, created_at }`
    (`Serialize` — metadata only, no `storage_key` on the wire);
  - `PhotoError` with `.field()` (drives `ApiError::Validation`).
- **`api::storage`** (the seam — infrastructure, no domain/HTTP):
  - `trait ObjectStore: Send + Sync` (`async_trait`): `put(key, &Bytes)`,
    `get(key) -> Bytes`, `delete(key)`, each `Result<_, ObjectStoreError>`;
  - `LocalObjectStore { root: PathBuf }` — writes `root/{key}` (key segments are
    UUIDs, so no traversal risk; still rejects `..`), `tokio::fs` IO; used by dev
    + the whole test suite. The **S3 impl is deferred to R-0026** (AC8) — the
    trait is the drop-in point.
- **`api::db`**: `PhotoSessionRow`/`PhotoRow` (`FromRow`) + `into_*` mappers (the
  corruption→500 discipline), and the queries (§2.5).
- **`api::photo`** (HTTP): the `Multipart` upload handler, the byte-streaming
  download handler, the metadata DTOs, and `routes()`. Validation is `core`'s;
  storage is the seam's; handlers are thin orchestration.
- **`AppState`** gains `store: Arc<dyn ObjectStore>`. `main.rs` builds a
  `LocalObjectStore` rooted at `PHOTO_STORE_ROOT` (env, default a data dir);
  tests build one in a `tempfile::TempDir`.

### 2.3 Upload (AC3/AC4/AC10 — bytes first, then row)

`POST /photo-sessions/:id/photos`, `Multipart` body:

1. **Authorize the session:** `find_session_by_id(pool, user, id)` → `404` if
   missing/foreign (before reading any bytes).
2. **Parse the multipart:** an optional text field `angle` and one file field.
   `axum::extract::Multipart` errors (malformed boundary, premature EOF, etc.)
   are caught and mapped to `ApiError::Validation { field: "file" }` by a small
   `multipart`-error→`ApiError` seam (the `http::parse_body` precedent — native
   axum rejections must **not** bypass the uniform error body; architect finding
   3). **Size has two cases:** the route carries
   `DefaultBodyLimit::max(MAX_BYTES + slack)`, so a body **beyond that limit**
   is rejected at the layer as **`413`** before the handler; a body **between
   `MAX_BYTES` and the slack** reaches the handler, is buffered, and its length
   check against `MAX_BYTES` yields **`400`** `{field:"file"}`. Content-type from
   the part header → `ImageContentType::parse` (non-image → `400`). Empty/no file
   → `400`.
3. **Validate** via `NewPhoto::new(angle, content_type, byte_size)` → `400`
   `{field}` on any rule.
4. **Write bytes first:** `photo_id = Uuid::new_v4()`,
   `key = format!("{user}/{session}/{photo}")`, `store.put(&key, &bytes)`.
   A store failure → `500` (no row written — nothing to dangle).
5. **Insert the row.** If the insert fails, **compensate**: `store.delete(&key)`
   (best-effort) then `500`. Order guarantees AC10: no metadata row ever points
   at absent bytes; the only failure residue is an orphaned object (compensated),
   never a dangling row.
6. `201` + the `SessionPhoto` metadata (never the bytes).

### 2.4 Download (AC6) & delete (AC7)

- `GET /photo-sessions/:id/photos/:photoId`: `find_photo(pool, user, session_id,
  photo_id)` (joins `photo_sessions` on `user_id`) → `404` if missing/foreign;
  else `store.get(&row.storage_key)` and return a `Response` with body = bytes,
  `Content-Type: row.content_type`, `Content-Length`. A store-miss (bytes gone)
  routes through `ApiError::from(ObjectStoreError) → ApiError::Internal` → `500`
  with the **opaque `{"error":"internal"}` body** (no new variant, no
  "bytes missing" leak; architect finding 2). No bytes are ever returned for a
  foreign photo.
- `DELETE …/photos/:photoId`: authorize → `store.delete(key)` then delete the
  row → `204`; missing/foreign/second-delete → `404`. (Delete bytes before the
  row so a row never outlives reachable bytes; a store failure → `500`, row
  kept, retryable.)
- `DELETE /photo-sessions/:id`: authorize → load the session's photo keys →
  `store.delete` each (best-effort, log failures) → delete the session row
  (FK cascade removes photo rows) → `204`. Foreign/missing → `404`.

### 2.5 Persistence

- `session_exists_for_user` (authorize path): `SELECT EXISTS(SELECT 1 FROM
  photo_sessions WHERE id = $1 AND user_id = $2)` — a lightweight ownership check
  the upload + photo-delete handlers use instead of `find_session_by_id`, which
  assembles the full photo list (architect finding 4; the workout
  `delete_session` scoped-query precedent).
- `insert_session` (AC2): plain `INSERT … RETURNING`; performed_on = today.
- `find_sessions_by_user` (AC5): sessions `WHERE user_id` `ORDER BY performed_on
  DESC, created_at DESC`, each with its photos (`ORDER BY created_at`), assembled
  in memory (the R-0004 nested-assembly precedent).
- `find_session_by_id` (AC5/auth): `WHERE id AND user_id` → `Option`.
- `insert_photo` (AC3): `INSERT … RETURNING` into `photo_session_photos`.
- `find_photo` (AC6/AC7): `SELECT p.* FROM photo_session_photos p JOIN
  photo_sessions s ON p.session_id = s.id WHERE p.id = $1 AND s.id = $2 AND
  s.user_id = $3` → `Option` (ownership via the join; cross-user → `None` →
  `404`).
- `delete_photo` / `delete_session`: `DELETE … WHERE …` scoped by owner;
  `rows_affected() > 0`.

Every id-addressed query carries `AND user_id = $caller` (directly or via the
join); no path id is trusted as an owner. Validation lives in `core`, never DB
`CHECK`s. `angle` is a nullable text column parsed back through `Angle::parse`
on read (corruption → 500).

### 2.6 Types & wire

- `Angle` ⇄ `front`/`back`/`left`/`right`/`other`.
- `ImageContentType` ⇄ `image/jpeg`/`image/png`.
- `SessionPhoto` JSON: `id`, `angle` (nullable), `content_type`, `byte_size`,
  `created_at` — **no `storage_key`** (internal only).
- `PhotoSession` JSON: `id`, `user_id`, `performed_on`, `photos[]`, `created_at`,
  `updated_at`.
- The upload response is the created `SessionPhoto`; the download response is raw
  bytes + `Content-Type` (not JSON).

### 2.7 Dependencies (new) — corrected per architect finding 1

The current workspace `tokio` features are
`["rt-multi-thread","macros","signal","net"]` (**no `fs`**) and `bytes` is **not**
a workspace dep — both are real gaps, added here, not assumed:

- enable the **`multipart`** feature on `axum` in `crates/api/Cargo.toml` (it is
  **not** a 0.7 default) — the `Multipart` extractor + `DefaultBodyLimit`;
- add **`fs`** to the workspace `tokio` features — `LocalObjectStore` IO;
- add **`bytes`** as an explicit workspace dep — the `ObjectStore` trait
  signatures name `Bytes` directly (not via the `axum::body::Bytes` re-export);
- add **`async-trait`** (workspace) — the `dyn ObjectStore` seam, depending on
  the crate directly (not the `axum::async_trait` re-export the extractor uses);
- add **`tempfile`** (api dev-dependency) — the test store root.

No new ML/image crates (AC12).

## 3. Code outline

Snippets representative; final form reconciled in step 5. Tests authored by `qa`
in step 3 against §6.

### 3.1 `core/src/photo.rs` (excerpt)

```rust
pub const MAX_BYTES: i64 = 10 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Angle { Front, Back, Left, Right, Other }
impl Angle { pub fn parse(s: &str) -> Result<Self, PhotoError> { /* … */ } }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ImageContentType { ImageJpeg, ImagePng }
impl ImageContentType {
    pub fn parse(s: &str) -> Result<Self, PhotoError> { /* image/jpeg|png */ }
    pub fn as_str(self) -> &'static str { /* … */ }
}

pub struct NewPhoto { pub angle: Option<Angle>, pub content_type: ImageContentType, pub byte_size: i64 }
impl NewPhoto {
    /// # Errors
    /// [`PhotoError`] on an out-of-range size (`1..=MAX_BYTES`).
    pub fn new(angle: Option<Angle>, content_type: ImageContentType, byte_size: i64)
        -> Result<Self, PhotoError> { /* size guard */ }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PhotoError {
    #[error("angle is not a known value")] AngleUnknown,
    #[error("content type must be image/jpeg or image/png")] ContentTypeUnsupported,
    #[error("image is empty or larger than the limit")] ByteSizeOutOfRange,
}
impl PhotoError { pub fn field(&self) -> &'static str { /* angle|content_type|file */ } }
```

### 3.2 `api/src/storage.rs` (the seam)

```rust
#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: &Bytes) -> Result<(), ObjectStoreError>;
    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError>;
    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError>;
}

pub struct LocalObjectStore { root: PathBuf }
// put: create_dir_all(root/parent), write(root/key, bytes)
// get: read(root/key) -> Bytes  (NotFound -> ObjectStoreError::Missing)
// delete: remove_file(root/key)  (NotFound is Ok — idempotent)
// rejects any key containing ".." (defense in depth; keys are UUID-only anyway)
```

The semantics — **`delete` is idempotent (missing key → `Ok`)** and **`get` on a
missing key → `ObjectStoreError::Missing`** — are part of the **trait contract
docs** (`# Errors` on each method), not just `LocalObjectStore` behaviour, so the
R-0026 S3 impl is held to the same contract (S3 `DeleteObject` is already
idempotent; `GetObject` 404 → `Missing`) — architect forward note.

### 3.3 `api/src/photo/handlers.rs` (upload, excerpt)

```rust
pub(crate) async fn upload(
    State(state): State<AppState>, user: AuthenticatedUser,
    Path(session_id): Path<Uuid>, mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<PhotoResponse>)> {
    db::find_session_by_id(&state.pool, user.user_id, session_id).await?
        .ok_or(ApiError::NotFound)?;                       // authorize first

    let (angle, content_type, bytes) = read_image_part(&mut multipart).await?; // 400 on shape
    let new = NewPhoto::new(angle, content_type, bytes.len() as i64)
        .map_err(|e| ApiError::Validation { field: e.field() })?;

    let photo_id = Uuid::new_v4();
    let key = format!("{}/{}/{}", user.user_id.0, session_id, photo_id);
    state.store.put(&key, &bytes).await.map_err(ApiError::from)?;     // bytes FIRST

    match db::insert_photo(&state.pool, session_id, photo_id, &new, &key).await {
        Ok(photo) => Ok((StatusCode::CREATED, Json(PhotoResponse::from(&photo)))),
        Err(e) => { let _ = state.store.delete(&key).await; Err(e) }  // compensate
    }
}
```

### 3.4 `AppState` + `lib.rs` + `main.rs`

```rust
pub struct AppState { pub pool: PgPool, pub jwt_secret: Arc<[u8]>, pub jwt_ttl: Duration,
                      pub store: Arc<dyn ObjectStore> }
// app(): .merge(photo::routes())  (upload route gets DefaultBodyLimit::max(MAX_BYTES + 1 MB))
// main(): store = Arc::new(LocalObjectStore::new(env PHOTO_STORE_ROOT or default))
```

## 4. Non-goals

Inherits R-0006 §4: no CV/pose/derived features, no real S3 (R-0026), no
presigned upload, no image transforms (verbatim bytes), no mobile UI, no per-day
uniqueness, no CDN/signed URLs. Also: no virus scanning, no rate limiting, no
image-dimension validation (only content-type + size).

## 5. Open questions

**Resolved by the architect review (2026-06-10, APPROVE WITH NITS).** All five
OQ-G approved as proposed. Four nits folded in above: §2.7 dependency
corrections (`tokio fs`, explicit `bytes`, axum `multipart` feature), §2.3 the
multipart-error→`400 {file}` mapper + the `413`-vs-`400` size split, §2.4 the
store-miss→opaque `Internal` mapping, §2.5 a lightweight `session_exists_for_user`
authorize query, and §3.2 the trait-contract semantics. **Tracked follow-up:** a
periodic orphan-object sweep (compensated orphans from row-insert failures) —
deferred, recorded so it is not lost.

- **OQ-G1 — `ObjectStore` via `async_trait` vs native async-fn-in-trait?**
  Proposed: `async_trait` (the `Arc<dyn …>` seam needs dyn-compatibility; native
  async fn in traits isn't `dyn`-safe without boxing).
- **OQ-G2 — Buffer the upload into `Bytes` vs stream to the store?** Proposed:
  buffer (cap 10 MB; `DefaultBodyLimit` guards the socket); streaming-to-store is
  a later optimization the seam allows without an API change.
- **OQ-G3 — Compensation vs transaction for upload (AC10)?** Proposed:
  bytes-first + best-effort `delete` on row-insert failure (object stores aren't
  transactional with Postgres); the residue is an orphaned object, never a
  dangling row. A periodic orphan-sweep is deferred.
- **OQ-G4 — Download as buffered `Bytes` vs `StreamBody`?** Proposed: buffered
  `Bytes` body + `Content-Type`/`Content-Length` (10 MB cap makes it safe);
  streaming is a later optimization.
- **OQ-G5 — Key scheme `{user}/{session}/{photo}` (all UUIDs).** Proposed:
  approve — non-guessable, user-namespaced, traversal-safe; `LocalObjectStore`
  still rejects `..` defensively.

## 6. Acceptance criteria

Each maps 1:1 to an R-0006 criterion and to the qa agent's tests.

- [ ] **SAC1 → AC1.** Migration creates `photo_sessions` + `photo_session_photos`
  with the FK cascades; user-delete cascades to both; session-delete cascades to
  photos; clean migration verified.
- [ ] **SAC2 → AC2.** `POST /photo-sessions` → `201` + empty session owned by the
  caller; `401`.
- [ ] **SAC3 → AC3.** Multipart upload of a JPEG/PNG → `201` + metadata + bytes
  in the store under a UUID key; non-image → `400`; oversized → `400`; missing
  file → `400`; foreign/missing session → `404`.
- [ ] **SAC4 → AC4.** A valid `angle` is stored; an unknown `angle` → `400`; an
  absent angle is allowed; a session accepts multiple photos.
- [ ] **SAC5 → AC5.** `GET /photo-sessions` → caller-only, newest-first, photos
  as metadata only (no `storage_key` key in the JSON); empty `[]`; `GET /:id`
  `200`/`404`.
- [ ] **SAC6 → AC6.** Download returns the exact bytes + the stored content-type
  for the owner; `404` for a foreign/missing photo; the upload→download round
  trip is byte-identical.
- [ ] **SAC7 → AC7.** Photo delete `204` + bytes gone from the store; second
  delete `404`; session delete `204` + all its bytes gone; foreign → `404`.
- [ ] **SAC8 → AC8.** The `ObjectStore` trait + `LocalObjectStore` exist; the
  whole suite runs with the local store and no cloud; a unit test covers
  put/get/delete + the missing-key and `..`-rejection paths.
- [ ] **SAC9 → AC9.** Cross-user `GET`/download/`DELETE` → `404`; no foreign
  bytes served; keys are UUID-only and user-namespaced (asserted on the stored
  key shape).
- [ ] **SAC10 → AC10.** A forced row-insert failure after a store write leaves
  **no row** and the object compensated (deleted); a store-write failure leaves
  no row. Error mapping pinned (`400`/`401`/`404`/`500`).
- [ ] **SAC11 → AC11.** Unit (`core` + `LocalObjectStore`) + `#[sqlx::test]`
  integration suites cover the surface; `cargo fmt`/`clippy`/`test`/`build` green.
- [ ] **SAC12 → AC12.** No image-processing/ML crate in the diff; no `mobile/`
  changes; only storage + metadata.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **`core::photo` pure domain; `api::storage` seam; `api::db` persistence; `api::photo` HTTP.** | Continues the R-0002→R-0005 layering; the storage seam is the one new inward-pointing infrastructure boundary. |
| 2026-06-10 | **`ObjectStore` trait (`put`/`get`/`delete`) + `LocalObjectStore`; S3 impl deferred to R-0026.** | Dependency-inverted bytes; dev/CI run cloud-free; S3 is a drop-in. (AC8/OQ-G1) |
| 2026-06-10 | **Bytes-first, row-second, compensate-on-failure (no upload transaction).** | Object stores aren't transactional with Postgres; this guarantees no dangling row, only a compensated orphan object. (AC10/OQ-G3) |
| 2026-06-10 | **Buffer uploads/downloads as `Bytes` under a 10 MB `DefaultBodyLimit`.** | Simple + safe at this size; streaming is a later optimization the seam permits. (OQ-G2/G4) |
| 2026-06-10 | **Key = `{user}/{session}/{photo}`, all UUIDs; `storage_key` never serialized.** | Non-guessable, user-namespaced, traversal-safe; health-data privacy. (AC9/OQ-G5) |
| 2026-06-10 | **`AppState` gains `store: Arc<dyn ObjectStore>`.** | One shared, cheaply-cloned seam handle, like `pool`. |
| 2026-06-10 | **(architect) Add `tokio fs` + explicit `bytes` + axum `multipart` feature; `async-trait`/`tempfile` new.** | The deps the spec first assumed present are genuinely missing; named precisely so step 5 doesn't stall. (finding 1) |
| 2026-06-10 | **(architect) Multipart shape errors → `400 {field:"file"}` via a `parse_body`-style mapper; oversized splits `413` (transport, beyond `DefaultBodyLimit`) vs `400` (buffered, over `MAX_BYTES`).** | Keeps the uniform error body; gives qa an exact code per case. (finding 3) |
| 2026-06-10 | **(architect) Download store-miss → `ApiError::Internal` opaque body; `session_exists_for_user` for the authorize path.** | No "bytes missing" leak; no needless photo-list load on the hot upload/delete path. (findings 2, 4) |

## Changelog

- _2026-06-10 — created (Draft). Realizes the accepted R-0006. Five HOW-level design questions (OQ-G1..G5) raised for the architect review; introduces the `ObjectStore` seam and the bytes-first upload discipline._
- _2026-06-10 — **Accepted.** Architect review returned APPROVE WITH NITS; all five OQ-G approved as proposed. Four nits applied in lockstep: §2.7 real dependency corrections (`tokio fs`, explicit `bytes`, axum `multipart`), §2.3 multipart-error→`400 {file}` mapper + `413`/`400` size split, §2.4 store-miss→opaque `Internal`, §2.5 `session_exists_for_user`, §3.2 trait-contract semantics (idempotent delete, `get`-NotFound→`Missing`). Orphan-sweep recorded as a deferred follow-up._
