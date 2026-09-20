# R-0024 — Privacy policy & health-data compliance

- **Status:** Draft
- **Milestone:** M8 — Launch readiness
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-09-20
- **Depends on:** nothing technical. It depends on the *inventory* being
            true, so it is written against the app as built (R-0002,
            R-0003, R-0004, R-0005, R-0006, R-0013, R-0032, R-0033).
- **Blocks:** R-0025 (store submission). Both stores refuse a listing
            without a reachable privacy-policy URL and a completed data
            disclosure form.
- **Realized by:** SPEC-0024 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

Goose Physics collects health data, body photographs and voice transcripts.
Before it is submitted to either store it must have (a) a published privacy
policy at a stable URL, (b) an accurate Apple *App Privacy* declaration and
Google *Data Safety* form, (c) in-app consent and deletion paths that match
what those documents claim, and (d) a legal review of the whole set against
Mexican LFPDPPP and the GDPR-adjacent regimes named in the source brief.

## 2. Rationale

This is the one launch item whose lead time is not ours to compress. The
store forms are sworn statements about data handling; an inaccurate one is
grounds for removal after launch, not just rejection before it. And the
categories this app touches — health, biometrics, photographs of the body —
are the ones both stores scrutinise hardest and most jurisdictions treat as
sensitive.

It is written now, ahead of the rest of M8, because it is the long pole.
Everything else on the M8 list is a day of work once a decision is made.

## 3. The inventory

What the app actually handles today, as built. The policy is written from
this table; if the table is wrong the policy is wrong.

| Data | Where it comes from | Where it goes | Third party |
|------|--------------------|---------------|-------------|
| Email, password hash (argon2id) | R-0002 register/login | Neon Postgres | Neon |
| Google account identity | R-0033 sign-in | Neon Postgres | Google |
| Age, height, weight, body stats, goals | R-0003 profile | Neon Postgres | Neon |
| Workout logs (exercise, load, reps, RPE) | R-0004, R-0009 | Neon Postgres | Neon |
| Nutrition logs (macros, calories) | R-0005, R-0010 | Neon Postgres | Neon |
| **Body photographs** | R-0006 upload | `ObjectStore` seam — local-fs today, S3 at R-0026 | depends on R-0026 |
| **Pose keypoints / frame features** derived from those photographs | R-0013 server-side estimation | Neon Postgres | Neon |
| **Voice transcripts** | R-0032. Speech→text runs **on device**; audio is never stored or transmitted | `POST /voice/intent`, then to an LLM for intent parsing | **Anthropic** (`claude-haiku-4-5`) |
| Derived program and diet proposals | R-0014, R-0017 | Neon Postgres | Neon |

Two entries in that table are the ones that will draw questions, and both
are answerable: audio never leaves the phone, and the photograph never
reaches the regression model — only features derived from it do
(`project-specifics.md`, "Photo pipeline feeds the structured model").

## 4. Acceptance criteria

- **AC1. Published policy.** A privacy policy is live at a stable URL on a
  domain we control, in **Spanish and English**, reachable without an
  account. Its data table is derived from §3, not from a template.
- **AC2. It names the processors.** Neon, Cloudflare, Google and Anthropic
  are named, with what each receives and why.
- **AC3. Apple App Privacy.** The declaration is completed and matches §3,
  including the *Health & Fitness*, *Photos*, *Audio Data* (none collected)
  and *Identifiers* categories, and whether any of it is linked to identity.
- **AC4. Google Data Safety.** The form is completed and matches §3,
  including the sensitive-data and third-party-sharing answers.
- **AC5. Deletion is real.** A user can delete their account and every
  artefact listed in §3 from inside the app, and the objects in the store
  go with the rows. If the sweep is asynchronous, the policy says so and
  states the window. (Note the open dependency: R-0028 exists precisely
  because orphaned objects can outlive their rows.)
- **AC6. Export.** A user can obtain their own data in a machine-readable
  form.
- **AC7. Consent before the sensitive paths.** Photograph upload and
  microphone use each carry an in-app disclosure at first use that states
  what is collected and where it goes, distinct from the OS permission
  prompt.
- **AC8. The policy is a checkable artefact.** A test asserts the in-app
  policy link resolves and that the app version's declared data categories
  match a committed manifest, so the two cannot drift silently.
- **AC9. Legal review.** A qualified reviewer — *not Claude, and not the
  owner acting as his own counsel* — signs off on the policy and the two
  store declarations against LFPDPPP and GDPR. The sign-off is recorded in
  §7.

## 5. Constraints & non-goals

- **Not legal advice.** This requirement organises the facts and names the
  gaps. AC9 exists because nothing here substitutes for counsel.
- No analytics or advertising SDK is in the app today, and none is added
  under this requirement — adding one later reopens every form above.
- No change to what the app collects. If the policy is uncomfortable to
  write, that is a signal to change the product under its own requirement,
  not to soften the wording.
- Minors are out of scope for v1: the policy states the service is not
  directed to under-18s, and onboarding does not ask for a birth date that
  would make us aware of one.

## 6. Open questions

- **OQ-1.** Data residency. Neon and Cloudflare place data outside Mexico.
  LFPDPPP permits transfer with disclosure; the policy must state where.
  Owner decision on whether that is acceptable for v1.
- **OQ-2.** Retention. How long do photographs survive after an account is
  closed? The answer is a number and it goes in the policy.
- **OQ-3.** Does the Anthropic call need its own consent, or is disclosure
  in the policy sufficient? Counsel's call, and it depends on OQ-1.
- **OQ-4.** Who is the data controller of record — a person or an entity?
  This also decides the Apple account type in R-0025, so it is worth
  settling once, here.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-20 | Requirement drafted against the as-built inventory rather than the source brief | The brief predates R-0032 and R-0033; a policy written from it would omit both Anthropic and Google |
