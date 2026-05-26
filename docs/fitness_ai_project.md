# FitnessAI — Adaptive Training & Nutrition App

Personalized fitness optimization using ML-driven adaptive programming based on individual physiological response.

---

## Project Overview

A cross-platform mobile app (iOS \+ Android) that collects user training logs, nutrition data, and progress photos, then uses a backend ML model to infer how the user's body responds to specific training stimuli and dynamically adjusts their program variables (volume, intensity, frequency, rest days, diet macros).

The intelligence is server-side. The mobile app stays lean — critical for the target market (Mexico, mixed Android hardware).

---

## Goals

- Let users log workouts, diet, and periodic progress photos  
- Automatically learn individual response patterns over time  
- Adapt training variables based on what's working for that specific body type  
- Use historical bodybuilder archetype data as initial priors (e.g. Mentzer, Arnold, Columbu, Yates — different genetics, different methods)  
- Monetize via monthly subscription (SaaS model)

---

## Architecture

Flutter App (iOS \+ Android)

        │

        │ HTTPS / REST

        ▼

  Rust Backend API (Axum or Actix-web \+ Tokio)

        │

        ├── User auth & subscription management

        ├── Log ingestion (workouts, diet, photos)

        ├── ML inference pipeline

        └── Program generation / adjustment engine

              │

              ▼

        ML Model (Rust — linfa → burn/tch-rs)

              │

              ▼

        PostgreSQL (user data, logs, metrics)

        Object Storage (progress photos)

---

## Tech Stack

### Mobile (Frontend)

- **Framework:** Flutter (Dart)  
- **Target:** iOS \+ Android from single codebase  
- **Philosophy:** Thin client — display, logging, photo capture only. No local inference.

### Backend

- **Language:** Rust  
- **HTTP Framework:** Axum (or Actix-web)  
- **Async Runtime:** Tokio  
- **Auth:** JWT-based (or OAuth2 if social login needed)  
- **Database:** PostgreSQL via `sqlx`  
- **Object Storage:** S3-compatible (for photos)  
- **Deployment:** Cloud-first MVP (AWS or Azure), self-hosted if unit economics demand later

### ML Pipeline

- **Language:** Rust  
- **Phase 1 (MVP):** `linfa` — regression/tree models on structured logs  
- **Phase 2:** `burn` or `tch-rs` — sequential/time-series modeling if needed  
- **Numerical compute:** `ndarray`  
- **Training data sources:**  
  - Historical bodybuilder methodology data (curated dataset)  
  - Anonymized user logs over time

---

## Core Features

### User-Facing

- [ ] Onboarding: body stats, goals, training history  
- [ ] Daily workout logger (exercises, sets, reps, weight, RPE)  
- [ ] Nutrition logger (macros, calories — manual entry \+ barcode scan)  
- [ ] Progress photo sessions (front, side, back, 3/4 — fixed-angle prompts)  
- [ ] Dashboard: trends, current program, weekly plan  
- [ ] Subscription paywall (monthly)

### Backend / ML

- [ ] User profile & archetype matching (initial prior from bodybuilder reference DB)  
- [ ] Log ingestion & time-series storage  
- [ ] Response inference: detect which variables correlate with positive outcomes  
- [ ] Program adjustment engine: tweak volume, frequency, intensity, rest days  
- [ ] Photo analysis: pose estimation \+ body composition segmentation (track physique change over time)  
- [ ] Compliance tracking: detect logging gaps, adjust confidence accordingly

---

## ML Model Design

### Input Features (per user, per time window)

- Training volume (sets × reps × weight per muscle group)  
- Training frequency (sessions/week per muscle group)  
- Intensity metrics (average RPE, % 1RM estimates)  
- Rest days between sessions  
- Macros: protein, carbs, fat, total calories  
- Sleep (optional, if user logs it)  
- Body measurements (weight, optional tape measures)  
- Photo-derived body comp proxy (extracted from images)

### Output / Predictions

- Predicted response: strength gain, body comp change  
- Recommended adjustments: volume up/down, frequency change, rest day insertion, macro shift

### Training Strategy

- **Phase 1:** Supervised regression on existing bodybuilder archetype data  
- **Phase 2:** Online/incremental learning per user as logs accumulate  
- **Prior initialization:** Match new user to closest archetype → inherit that program as starting point, then personalize

### Photo Pipeline

- Fixed-angle photo sessions → pose estimation (e.g. MediaPipe or custom model)  
- Extract: shoulder width proxy, muscle belly visibility, symmetry score  
- Feed structured photo-derived features into main model (not raw images)

---

## Data Model (High Level)

User

  ├── Profile (age, height, weight, goals, body stats)

  ├── WorkoutLogs (date, exercises\[\], sets\[\], reps\[\], weight\[\], rpe\[\])

  ├── NutritionLogs (date, protein, carbs, fat, calories)

  ├── PhotoSessions (date, angle\[\], image\_ref\[\], derived\_metrics{})

  ├── ProgramHistory (program\_version, start\_date, adjustments\[\])

  └── Subscription (plan, status, billing\_cycle)

ArchetypeLibrary

  ├── Name (e.g. "Mentzer", "Arnold", "Columbu")

  ├── TrainingMethod (volume, frequency, intensity profile)

  ├── BodyType (muscle insertions, genetic profile description)

  └── OutcomeData (what worked, what didn't)

---

## Deployment Plan

### MVP (Phase 1\)

- Cloud: AWS or Azure  
- Single Rust API server (containerized via Docker)  
- Managed PostgreSQL (RDS or Azure Database)  
- S3 or Azure Blob for photo storage  
- CI/CD: GitHub Actions → Docker Hub → cloud

### Scale / Cost Optimization (Phase 2+)

- Evaluate self-hosted VPS (Hetzner, DigitalOcean) for better margins  
- Add GPU instance if on-device or server-side inference needs acceleration  
- CDN for static assets

---

## Monetization

- Monthly subscription (SaaS)  
- Freemium tier: manual logging only, no AI adjustment  
- Paid tier: full adaptive AI, photo analysis, personalized program  
- Target market: Mexico \+ LATAM initially

---

## Open Questions / TODOs

- [ ] What subscription price point works for LATAM market?  
- [ ] Which pose estimation model to use for photo analysis (MediaPipe, custom)?  
- [ ] How to handle low-compliance users (logging gaps degrade model signal)  
- [ ] App Store \+ Play Store developer accounts needed  
- [ ] Legal: privacy policy, data handling for health data (photos, biometrics)  
- [ ] Decide on social login vs email auth for onboarding friction reduction

---

## Project Name (TBD)

Working title: **FitnessAI** — rename before launch  
