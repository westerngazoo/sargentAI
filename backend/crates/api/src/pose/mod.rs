//! The pose-estimation seam (R-0013, SPEC-0013 §2.4/§2.6).
//!
//! [`PoseEstimator`] abstracts "image bytes → COCO-17 keypoints"; the endpoint
//! depends on `Arc<dyn PoseEstimator>` (in `AppState`), exactly as R-0006's
//! handlers depend on `Arc<dyn ObjectStore>`. Two implementations:
//! [`OnnxPoseEstimator`] (the real in-process `MoveNet` model via `ort`) and
//! [`FakePoseEstimator`] (deterministic, used by the integration suite so it
//! never loads the model).

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use image::imageops::FilterType;
use ort::{session::Session, value::Tensor};
use thiserror::Error;

use fitai_core::pose::{Keypoint, PoseKeypoints};
use fitai_core::ImageContentType;

/// A pose-estimation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PoseError {
    /// No keypoint cleared the confidence floor — no usable pose in the image.
    #[error("no person detected in the image")]
    NoPersonDetected,
    /// The stored bytes could not be decoded as an image.
    #[error("could not decode the image bytes")]
    Decode,
    /// The model or runtime faulted.
    #[error("pose inference failed")]
    Inference,
}

/// Server-side pose estimation: image bytes → a COCO-17 [`PoseKeypoints`].
#[async_trait]
pub trait PoseEstimator: Send + Sync {
    /// Estimate a pose from encoded image bytes.
    ///
    /// # Errors
    /// [`PoseError`] when the image cannot be decoded, no person is detected, or
    /// inference faults.
    async fn estimate(
        &self,
        bytes: &[u8],
        content_type: ImageContentType,
    ) -> Result<PoseKeypoints, PoseError>;
}

// ===========================================================================
// The real `MoveNet` estimator (SPEC-0013 §2.6).
// ===========================================================================

/// `MoveNet` `SinglePose` Lightning, embedded and run in-process via ONNX Runtime.
pub struct OnnxPoseEstimator {
    // `Session::run` needs `&mut self`; `Session` is `Send + Sync`, so a single
    // model is shared behind a `Mutex` and locked per inference (in a blocking
    // task). One image per match makes serialized inference a non-issue.
    session: Arc<Mutex<Session>>,
}

/// The bundled `MoveNet` `SinglePose` Lightning model, **fp32** (Apache-2.0; see
/// `models/LICENSE`). Embedded so there is no runtime file I/O (SPEC-0013 §2.6).
/// fp32, not fp16: ONNX Runtime's CPU kernels emit garbage for the fp16 export.
static MODEL: &[u8] = include_bytes!("../../models/movenet-lightning.onnx");

/// `MoveNet` Lightning's square input edge, in pixels.
const INPUT_EDGE: u32 = 192;
/// Keypoints below this score are not a confident detection.
const DETECTION_FLOOR: f32 = 0.2;

impl OnnxPoseEstimator {
    /// Load the bundled model into a shared session.
    ///
    /// # Errors
    /// [`PoseError::Inference`] if the ONNX Runtime cannot build a session from
    /// the embedded model.
    pub fn load() -> Result<Self, PoseError> {
        let mut builder = Session::builder().map_err(|_| PoseError::Inference)?;
        let session = builder
            .commit_from_memory(MODEL)
            .map_err(|_| PoseError::Inference)?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }
}

#[async_trait]
impl PoseEstimator for OnnxPoseEstimator {
    async fn estimate(
        &self,
        bytes: &[u8],
        _content_type: ImageContentType,
    ) -> Result<PoseKeypoints, PoseError> {
        let input = preprocess(bytes)?;
        let session = Arc::clone(&self.session);
        // Inference is CPU-bound and `run` is blocking — keep it off the async
        // runtime's worker threads.
        let raw = tokio::task::spawn_blocking(move || run_model(&session, input))
            .await
            .map_err(|_| PoseError::Inference)??;
        parse_keypoints(&raw)
    }
}

/// Decode and letterbox the image to `MoveNet`'s NHWC `int32` 0–255 input
/// (SPEC-0013 §2.6). Letterboxing preserves aspect ratio with a uniform scale,
/// so the derived span *ratios* are invariant to it.
fn preprocess(bytes: &[u8]) -> Result<Vec<i32>, PoseError> {
    #![allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )] // pixel/coordinate arithmetic over bounded image dimensions.
    let image = image::load_from_memory(bytes)
        .map_err(|_| PoseError::Decode)?
        .to_rgb8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(PoseError::Decode);
    }

    let scale = (INPUT_EDGE as f32 / width as f32).min(INPUT_EDGE as f32 / height as f32);
    let new_w = (width as f32 * scale).round().max(1.0) as u32;
    let new_h = (height as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&image, new_w, new_h, FilterType::Triangle);

    // Centre the aspect-preserved image on a black square canvas.
    let mut canvas = image::RgbImage::from_pixel(INPUT_EDGE, INPUT_EDGE, image::Rgb([0, 0, 0]));
    let offset_x = i64::from((INPUT_EDGE - new_w) / 2);
    let offset_y = i64::from((INPUT_EDGE - new_h) / 2);
    image::imageops::overlay(&mut canvas, &resized, offset_x, offset_y);

    // NHWC, channel order R,G,B, values 0..=255 — no normalization.
    let mut tensor = Vec::with_capacity((INPUT_EDGE * INPUT_EDGE * 3) as usize);
    for pixel in canvas.pixels() {
        tensor.push(i32::from(pixel[0]));
        tensor.push(i32::from(pixel[1]));
        tensor.push(i32::from(pixel[2]));
    }
    Ok(tensor)
}

/// Run the session over the prepared NHWC tensor and return the flat output.
fn run_model(session: &Mutex<Session>, input: Vec<i32>) -> Result<Vec<f32>, PoseError> {
    let edge = i64::from(INPUT_EDGE);
    let tensor =
        Tensor::from_array(([1_i64, edge, edge, 3], input)).map_err(|_| PoseError::Inference)?;
    let mut guard = session.lock().map_err(|_| PoseError::Inference)?;
    let outputs = guard
        .run(ort::inputs![tensor])
        .map_err(|_| PoseError::Inference)?;
    let (_shape, data) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|_| PoseError::Inference)?;
    Ok(data.to_vec())
}

/// Parse `MoveNet`'s `[1,1,17,3]` `(y, x, score)` output into [`PoseKeypoints`].
fn parse_keypoints(raw: &[f32]) -> Result<PoseKeypoints, PoseError> {
    const COCO17: usize = 17;
    if raw.len() < COCO17 * 3 {
        return Err(PoseError::Inference);
    }
    let points: [Keypoint; COCO17] = std::array::from_fn(|i| Keypoint {
        // `MoveNet` emits (y, x, score); our Keypoint is (x, y, score).
        x: raw[i * 3 + 1],
        y: raw[i * 3],
        score: raw[i * 3 + 2],
    });
    if points.iter().all(|k| k.score < DETECTION_FLOOR) {
        return Err(PoseError::NoPersonDetected);
    }
    Ok(PoseKeypoints::new(points))
}

// ===========================================================================
// The deterministic test fake (kept in the public API so the integration suite,
// a separate crate, can inject it — mirrors `LocalObjectStore`).
// ===========================================================================

/// A pose estimator that returns a pre-configured result, ignoring its input.
/// Lets the match suite drive the endpoint with known keypoints (or a
/// [`PoseError`]) without loading the model.
pub struct FakePoseEstimator {
    result: Result<PoseKeypoints, PoseError>,
}

impl FakePoseEstimator {
    /// A fake that returns the given keypoints for every image.
    #[must_use]
    pub fn returning(keypoints: PoseKeypoints) -> Self {
        Self {
            result: Ok(keypoints),
        }
    }

    /// A fake that fails with the given error for every image.
    #[must_use]
    pub fn failing(error: PoseError) -> Self {
        Self { result: Err(error) }
    }
}

impl Default for FakePoseEstimator {
    /// The default fake reports no detection — suites that never reach the match
    /// endpoint use it only to satisfy the `AppState` field.
    fn default() -> Self {
        Self {
            result: Err(PoseError::NoPersonDetected),
        }
    }
}

#[async_trait]
impl PoseEstimator for FakePoseEstimator {
    async fn estimate(
        &self,
        _bytes: &[u8],
        _content_type: ImageContentType,
    ) -> Result<PoseKeypoints, PoseError> {
        self.result.clone()
    }
}
