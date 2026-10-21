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
| **Date of birth** (age derived; minimum accepted age 13 today), height, weight, body stats, goals | R-0003 profile | Neon Postgres | Neon |
| Workout logs (exercise, load, reps, RPE) | R-0004, R-0009 | Neon Postgres | Neon |
| Nutrition logs (macros, calories) | R-0005, R-0010 | Neon Postgres | Neon |
| **Body photographs** | R-0006 upload | `ObjectStore` seam — the API container's own disk today (not durable across restarts), S3 at R-0026 | Cloudflare (host); later R-0026's store |
| **Pose keypoints / frame features** derived from those photographs | R-0013 server-side estimation | Neon Postgres | Neon |
| **Voice audio** | R-0032. Recognised by the platform speech service (`speech_to_text`, `onDevice` not set), which may send audio to **Apple** (iOS) or **Google** (Android); the app itself never stores or uploads audio | the OS recognizer | Apple / Google (OS speech service) |
| **Voice transcripts** | the recognizer's text | `POST /voice/intent`, then to an LLM for intent parsing | **Anthropic** (`claude-haiku-4-5`) |
| **All API traffic** (auth, profile, logs, transcripts, photo uploads) | every request | the API, hosted on Cloudflare Workers + Containers | **Cloudflare** |
| Derived program and diet proposals | R-0014, R-0017 | Neon Postgres | Neon |

Three entries in that table will draw questions. The photograph never
reaches the regression model; only features derived from it do
(`project-specifics.md`, "Photo pipeline feeds the structured model").
Audio is **not** guaranteed to stay on the phone today, because the OS
recognizer may send it to Apple or Google (OQ-5). And the profile stores a
date of birth and accepts users from 13 (OQ-6). *(These three rows were
corrected on 2026-10-20 against the code; the draft had said audio never
leaves the phone, that no birth date is asked, and had omitted Cloudflare.)*

## 4. Acceptance criteria

- **AC1. Published policy.** A privacy policy is live at a stable URL on a
  domain we control, in **Spanish and English**, reachable without an
  account. Its data table is derived from §3, not from a template.
- **AC2. It names the processors.** Neon, Cloudflare, Google and Anthropic
  are named, with what each receives and why.
- **AC3. Apple App Privacy.** The declaration is completed and matches §3,
  including the *Health & Fitness*, *Photos*, *Audio Data* (per OQ-5) and
  *Identifiers* categories, and whether any of it is linked to identity.
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
- Minors: today onboarding **does** ask for a date of birth and admits ages
  13 and up (`MIN_AGE = 13` in `profile.rs`, "pending the M8 legal review").
  Whether v1 serves 13–17 is OQ-6; either answer is a product change under
  its own requirement, not a softer sentence here.

## 6. Open questions

- **OQ-1.** Data residency. Neon and Cloudflare place data outside Mexico.
  LFPDPPP permits transfer with disclosure; the policy must state where.
  Owner decision on whether that is acceptable for v1.
- **OQ-2.** Retention. How long do photographs survive after an account is
  closed? The answer is a number and it goes in the policy.
- **OQ-3.** Does the Anthropic call need its own consent, or is disclosure
  in the policy sufficient? Counsel's call, and it depends on OQ-1.
- **OQ-5.** Voice audio: force on-device recognition (`onDevice: true`,
  with a fallback for devices without it), or disclose the OS recognizer
  and declare *Audio Data* accordingly? Counsel weighs in on whether OS-
  service recognition is the app's collection.
- **OQ-6.** Minors: raise the floor to 18, or serve 13–17 with the minors'
  provisions (consent, retention, store age ratings)?
- **OQ-4.** Who is the data controller of record — a person or an entity?
  This also decides the Apple account type in R-0025, so it is worth
  settling once, here.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-20 | Requirement drafted against the as-built inventory rather than the source brief | The brief predates R-0032 and R-0033; a policy written from it would omit both Anthropic and Google |
| 2026-10-20 | Three inventory rows corrected against the code before merge | The draft said audio never leaves the phone (the OS recognizer may send it out), that no birth date is asked (one is, from age 13), and omitted Cloudflare (it hosts the API). The decisions these raise are OQ-5 and OQ-6. |
