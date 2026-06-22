# Agent Task — Research: Cloud ML Model Training Architecture

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** none — research only
**Output:** write your findings to `docs/ml-cloud-training-research.md` and commit to `main`

---

## What you are doing

You are a research agent. Do NOT write any implementation code. Produce a
technical research note that will be used to make decisions for R-0015/R-0016
(adaptive ML model, Milestone M5).

---

## Step 1 — Read these files first

```
project-specifics.md                          ← stack, infra target (AWS/Azure), ML plan
ROADMAP.md                                    ← where R-0015/R-0016 sit in the sequence
requirements/0015-log-aggregation.md          ← if it exists
requirements/0016-response-inference.md       ← if it exists
```

---

## Step 2 — Context

fitAI is a Rust backend + Flutter mobile fitness app. The ML roadmap is:

**Phase 1 (MVP):** `linfa` (Rust ML crate) — regression + tree models trained
on structured workout logs (sets, reps, weight, body metrics over time).
Predicts strength gain and body-composition change; recommends adjustments to
volume, frequency, intensity, rest, and macros.

**Phase 2:** `burn` or `tch-rs` — sequential/time-series models for richer
per-user adaptation.

**Infrastructure:** Docker on AWS or Azure (decision not yet made). CPU-only
for MVP (no GPU budget). The model is trained periodically on accumulated user
logs and served by the same Rust API process.

**Scale assumption for research:** <1,000 users for MVP/early launch.

---

## Step 3 — Research questions to answer

Answer each question concisely. Cite sources (crate docs, pub.dev, GitHub issues,
official AWS/Azure docs, arxiv if relevant).

### 1. linfa in production

- Is `linfa` batch-only or does it support incremental/online learning?
- Realistic re-training time for linear regression or gradient-boosted trees on
  ~10,000 rows × ~20 features on a single CPU core?
- Can a trained `linfa` model artifact be serialized (e.g. via `serde` + bincode
  or JSON) and hot-swapped at runtime without a server restart?
- Is `linfa` production-mature enough for MVP? Any known gaps or limitations?

### 2. burn vs tch-rs for Phase 2

- CPU inference performance — which is faster for small sequential models
  (~LSTM / TCN size) on a single CPU core?
- ONNX export support — we already use ONNX Runtime for pose estimation (MediaPipe).
  Can either `burn` or `tch-rs` export models to ONNX for a unified serving path?
- Maintenance activity and community health as of mid-2026?
- Recommendation: which should we choose for Phase 2, and why?

### 3. Cloud training options

Evaluate these four options for scheduling nightly model retraining at <1,000
users. For each, assess: **cost** (monthly USD estimate at this scale),
**operational complexity** (1 = trivial, 5 = needs a DevOps team), and
**Rust fit** (how naturally the Rust binary slots in).

| Option | Description |
|--------|-------------|
| A | Nightly cron on the same server: `cargo run --bin retrain` |
| B | AWS SageMaker training job triggered by CloudWatch Events |
| C | AWS Fargate task triggered by CloudWatch Events (or equivalent Azure Container Instance) |
| D | Fly.io machine spun up on demand for the retrain job |

End with a recommendation.

### 4. Model versioning + hot-swap

How should the trained model artifact be stored and versioned so the live API
server can load a new model with zero downtime?

Evaluate:
- S3 (or Azure Blob) artifact store + the API server polling for a new version
- Blue-green model pointers in the database (the server checks a `model_version`
  table row at startup or on a timer)
- Atomic file swap on the server's local disk

What's the minimum viable approach for a team of one?

### 5. Privacy — training data compliance

Users' workout logs are the training data. What is the minimum required for:
- **GDPR** (relevant for any EU users)
- **Mexico's LFPDPPP / LATAM** (the primary target market)

Cover:
- Anonymization approach before training (pseudonymization vs aggregation)
- Data retention policy that satisfies right-to-erasure without corrupting the
  trained model
- Whether a model trained on a user's data must be retrained/pruned if that
  user deletes their account ("right to be forgotten" machine-unlearning problem)

---

## Step 4 — Write the output

Write a structured markdown file at `docs/ml-cloud-training-research.md`.
Structure:

```
# ML Cloud Training — Research Note
Date: 2026-06-22

## 1. linfa in production
## 2. burn vs tch-rs
## 3. Cloud training options
## 4. Model versioning
## 5. Privacy compliance
## Recommendations
```

Keep the total length to 600–900 words. The Recommendations section is the most
important — make clear, opinionated choices.

---

## Step 5 — Commit

```bash
git checkout main
git add docs/ml-cloud-training-research.md
git commit -m "docs: ML cloud training research note for R-0015/R-0016"
git push
```
