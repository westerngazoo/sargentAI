# R-0041 — Authored Program Persistence & Serving

- **Status:** Draft
- **Milestone:** M-Platform (trainer marketplace) — layer A (authoring), part 2
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-29
- **Depends on:** R-0039 (the `core::authoring` model + `materialize` this stores
                  and serves), R-0038 (load math), R-0015 (per-lift e1RM — the
                  load anchor for materialization)
- **Realized by:** SPEC-0041 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Persist and serve the authored programs that R-0039 can already model. A user
(trainer or self-author) can **save** an `AuthoredProgram`, **list** their own,
**fetch** one, and **fetch it materialized** against their own e1RM — so the
mobile app has something to read and the builder UI (a later requirement) has an
API to write to.

R-0039 deliberately shipped as pure domain logic: SPEC-0039 §4 lists persistence
as an explicit non-goal. This requirement is that non-goal, promoted.

## 2. Rationale

`core::authoring` is complete, tested and on `main`, but nothing can reach it —
there is no endpoint, so no client can save or read an authored program. This is
the thinnest slice that turns the merged domain model into a usable feature, and
every later platform layer (assignment to clients, trainer visibility,
subscriptions) needs programs to exist in the database first.

## 3. Acceptance criteria

- **AC1. Storage.** An `authored_programs` table keyed by a server-generated
  UUID, owned by a user, holding the program `name` and the serialized
  `AuthoredProgram` as JSONB, with `created_at` / `updated_at`.
- **AC2. Create.** `POST /authored-programs` accepts an `AuthoredProgram`,
  **validates it through `core::authoring`** before storing, and returns the
  stored program with its id and `201`.
- **AC3. Validation is typed and specific.** An invalid program is rejected with
  **422** and a body naming the failing reason, derived from the typed
  `AuthorError` — never a 500, never a generic message. Validation happens
  server-side regardless of what the client claims.
- **AC4. List.** `GET /authored-programs` returns only the caller's own
  programs, newest first.
- **AC5. Fetch one.** `GET /authored-programs/{id}` returns the program.
- **AC6. Ownership is enforced as absence.** A program owned by another user
  responds **404**, not 403 — the API must not disclose that an id exists.
- **AC7. Materialize.** `GET /authored-programs/{id}/materialized` runs
  `materialize(program, e1rm, plate_kg)` with **the caller's own e1RM**, derived
  from R-0015's aggregation over the caller's logged sessions, and returns the
  concrete cycle with plate-rounded loads. A lift the caller has not trained
  **within the 8-week window** yields a `null` load rather than an error or a
  fabricated number — a stale e1RM is a worse load anchor than none.
- **AC8. Authentication.** Every endpoint requires a valid JWT; unauthenticated
  requests are **401**.
- **AC9. Round-trip fidelity.** A program written and read back is byte-for-byte
  equivalent as a domain value — `AuthoredProgram` in, identical
  `AuthoredProgram` out, no field silently dropped or re-ordered into a
  different meaning.
- **AC10. Tests.** Integration tests covering: auth required; create happy path;
  each distinct validation rejection; list scoped to owner; fetch-one; a
  non-owner receiving 404; materialize with e1RM present; materialize with a
  lift that has no e1RM (the `null`-load path); and the AC9 round-trip.

## 4. Constraints & non-goals

- Persistence + read endpoints only. **Explicitly out of scope:**
  - The mobile **program-builder UI** (#79 — its own requirement).
  - **Update / delete** of an authored program (v1 is create + read; mutation
    is a later requirement once the builder UI defines the editing model).
  - **Roles** (trainer/client/self — R-0040), assignment to clients, trainer
    visibility into someone else's program.
  - **Community/roster**, **subscriptions/billing** (M7).
  - Program-tied meal plans; weekday/date calendars.
  - ~~Any change to `core::authoring` itself.~~ **Amended 2026-08-04:** one
    refactor of `core::authoring` is in scope — extracting the existing
    validation behind a public `validate` entry point, over a shared `index()`
    helper that `materialize` also uses. No behavioural change. This was forced
    by architect review: the private `validate` cannot be exported as-is
    (it takes an index map and `plate_kg`, and two of the eleven error variants
    are raised outside it), and validating by calling `materialize` would make
    `POST`'s coverage a side effect of another function's implementation.
    `core::aggregate` also gains `current_e1rm` + `DEFAULT_WINDOW_WEEKS`.
- `plate_kg` is a request-level concern for materialization, defaulted
  server-side; it is not stored on the program.

## 5. Open questions (deferred to SPEC-0041)

- **OQ-1:** Is `plate_kg` a query parameter on `/materialized` with a server
  default, or fixed server-side in v1? (Lean: query parameter, defaulted, so a
  kg/lb gym is a client concern rather than a migration.)
- **OQ-2:** Does the stored row denormalize anything for cheap listing (e.g.
  cycle length, exercise count), or does `GET /authored-programs` deserialize
  every program to build the list? (Lean: return a lightweight summary shape for
  the list and the full program only on fetch-one.)
- **OQ-3:** Is a per-user uniqueness constraint on program `name` wanted, or are
  duplicate names allowed? (Lean: allow duplicates; the id is the identity.)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-29 | Promote SPEC-0039's persistence non-goal to its own requirement | Constitution §1 — the endpoint work needs an accepted requirement + spec of its own rather than riding R-0039's. |
| 2026-07-29 | Create + read only in v1; no update/delete | The editing model belongs with the builder UI; shipping read paths unblocks the client now. |
| 2026-07-29 | Non-owner access returns 404, not 403 | Ownership is modelled as absence so ids are not enumerable. |
| 2026-07-29 | Validate server-side through `core::authoring` on every write | The client is untrusted; one validation path shared with the domain model. |

## Changelog

- _2026-07-29 — created (Draft)._
