# ML Cloud Training — Research Note
Date: 2026-06-22

## 1. linfa in production

- **Batch vs Online Learning:** `linfa` is primarily a batch-learning framework. Its core design (traits like `Fit`) expects all data in memory (`DatasetBase`) at once. It does not have built-in, native traits for true online/incremental learning (e.g., streaming rows one by one to update weights) for most of its algorithms. For regression or tree models, you typically re-fit from scratch.
- **Re-training time:** For ~10,000 rows × ~20 features on a single CPU core, training a linear regression model or a small random forest/gradient-boosted tree (if using a crate that ports trees well, though native `linfa-trees` supports decision trees and `linfa-ensemble` supports random forests) is extremely fast. Expect it to take a few milliseconds to a couple of seconds at most. It is fully viable to just re-run batch training on CPU nightly.
- **Serialization and hot-swapping:** Yes, trained `linfa` models implement `serde::Serialize` and `serde::Deserialize`. They can be serialized to JSON, `bincode`, or CBOR, stored on disk/S3, and loaded back into memory by the API server dynamically.
- **Production maturity:** `linfa` is relatively mature but development has slowed in recent years. It is excellent for MVP (simple linear/logistic regression, K-means, SVMs, trees) and very lightweight. The main limitation is its smaller ecosystem compared to Python's scikit-learn (fewer algorithms, fewer hyperparameter-tuning utilities), but for a structured-log regression MVP, it is perfectly adequate.

## 2. burn vs tch-rs

- **CPU Inference Performance:** For small sequential models (like small LSTMs or Temporal Convolutional Networks) on a single CPU core, `tch-rs` (which wraps the C++ libtorch/PyTorch backend) is generally highly optimized through deep vectorization (MKL/OpenBLAS). `burn` with its `NdArray` backend is pure Rust and is catching up rapidly in CPU performance, but `tch-rs` still holds an edge in raw, highly optimized CPU execution for complex RNN/sequential ops. However, `burn`'s WGPU backend is great if GPUs were available.
- **ONNX Export:** We already use ONNX Runtime. `burn` supports *importing* ONNX models but its ONNX *export* capabilities are limited and a work in progress. `tch-rs` doesn't export to ONNX natively from Rust easily (it relies on PyTorch Python for JIT/ONNX tracing typically). If a unified ONNX serving path is critical, it's often easier to train in Python and export to ONNX, or use `burn` which is actively moving towards ONNX export.
- **Maintenance Health:** As of mid-2026, `burn` is highly active, idiomatically Rust, and growing a vibrant community. `tch-rs` is mature and stable but is inherently a complex C++ wrapper that can cause build/linking friction.
- **Recommendation:** **Choose `burn` for Phase 2.** Even if CPU inference is slightly slower than `tch-rs`, for <1,000 users and small models, the difference is negligible. `burn` provides a pure-Rust, seamless build experience (no C++ libtorch linking headaches), fitting perfectly into a Rust API monorepo. Its active development means it will likely support your exact needs by the time Phase 2 begins.

## 3. Cloud training options

| Option | Cost (USD/mo) | Complexity | Rust Fit |
|---|---|---|---|
| A. Same server cron | ~$0 (uses existing API server compute) | 1 (trivial, just `tokio::spawn` or a `cron` crate) | Excellent. Just a function call or local binary. |
| B. SageMaker + CloudWatch | ~$50+ (instance minimums, ECR, orchestration) | 4 (requires IAM, custom containers, step functions) | Poor. SageMaker expects Python scripts; Rust containers require custom BYOC (Bring Your Own Container) setups. |
| C. Fargate task | ~$2 - $5 (pay per minute of compute) | 3 (ECS setup, EventBridge rules, IAM roles) | Good. Rust compiles to small, fast-starting Docker images perfect for ephemeral Fargate tasks. |
| D. Fly.io Machines | < $1 (pay per second) | 2 (simple `flyctl config`, HTTP API to spin up) | Excellent. Very fast boot times for Rust binaries. |

**Recommendation:** **Option A (Nightly cron on the same server) for MVP.** With <1,000 users and `linfa` models training in seconds on CPU, there is absolutely no need to overcomplicate architecture. Run training asynchronously on the API server. When you outgrow this (e.g., Phase 2 memory/compute constraints), move to Option C (Fargate) or Option D (Fly.io). Avoid SageMaker for a Rust stack.

## 4. Model versioning

- **S3 / Blob artifact store:** Standard and scalable. The server polls S3 or receives a webhook. However, it requires setting up S3 and polling logic.
- **Blue-green DB pointers:** The model artifact is either in the DB (if small) or S3, but the "active" version is a row in PostgreSQL. The server checks this on a timer. Very robust.
- **Atomic file swap:** The training job writes `model_v2.bin`, symlinks `model_latest.bin -> model_v2.bin` (or does atomic rename `mv -T`), and the API server either watches the filesystem (`notify` crate) or reloads periodically.

**Recommendation:** **Atomic file swap on the local disk.** Since we selected Option A for training (same server), the easiest and most robust MVP approach for a team of one is saving the `serde`-serialized `linfa` model to the local filesystem and using atomic renames. The Axum API can hold an `Arc<RwLock<Model>>` and a background Tokio task can periodically check the file modified time, updating the lock with zero downtime.

## 5. Privacy compliance

- **GDPR & LFPDPPP Minimums:** Workout logs and physiological data are health/biometric data. You need explicit user consent, a privacy policy detailing data usage for ML, and strict access controls.
- **Anonymization:** For training, aggregate or pseudonymize. Strip all identifiers (names, emails) before feeding rows into the `linfa` dataset. Instead of `user_id=123`, the model should ideally learn from cross-user aggregated statistics if possible, or train per-user models that are logically separated. If training a global model, the input rows should not contain PII.
- **Data Retention & Erasure:** If a user exercises their "right to be forgotten", their row in the `users` and `workout_logs` tables must be deleted.
- **Machine Unlearning:** Do you need to unlearn the model? Generally, no. Under current GDPR interpretations (and LATAM laws which are usually less strict than GDPR), if the training data was properly anonymized/pseudonymized before training, the resulting model weights are not considered personal data. You do *not* need to immediately retrain the global model to "forget" their anonymized data contribution. Simply deleting their original database records suffices. When the model next retrains (e.g., the nightly job), it will naturally exclude the deleted user's data.

## Recommendations

1. **Phase 1 ML:** Use `linfa`. It trains fast enough on CPU for batch retraining to be viable. Serialize models via `bincode`.
2. **Phase 2 ML:** Choose `burn`. Pure Rust, actively maintained, and avoids C++ linking complexity.
3. **Training Infra:** Use **Option A** (Same server background job/cron). For <1,000 users, CPU training takes seconds. Keep it monolithic.
4. **Model Versioning:** Use **Atomic file swap** with an `Arc<RwLock<Model>>` in Axum, reloaded by a background Tokio task watching the file.
5. **Privacy:** Strip PII before training. You do not need to complexly "unlearn" models on deletion; deleting the user's DB records ensures they are excluded from the next nightly batch retrain.
