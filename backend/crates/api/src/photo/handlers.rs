//! HTTP handlers for the photo-session endpoints.
//!
//! Handlers are thin orchestration: validation is `core`'s job, persistence is
//! `db`'s, and the bytes go through the `storage` seam. The upload writes bytes
//! **before** the metadata row and compensates on failure, so a row never points
//! at absent bytes (SPEC-0006 §2.3, AC10). Multipart and store errors are mapped
//! to the uniform `ApiError` shapes (never axum's native rejection).

use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use chrono::Utc;
use uuid::Uuid;

use fitai_core::{Angle, ImageContentType, NewPhoto, PhotoSession, SessionPhoto};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    AppState,
};

pub(crate) async fn create_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<(StatusCode, Json<PhotoSession>)> {
    let session =
        db::insert_photo_session(&state.pool, user.user_id, Utc::now().date_naive()).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub(crate) async fn list_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<PhotoSession>>> {
    let sessions = db::find_photo_sessions_by_user(&state.pool, user.user_id).await?;
    Ok(Json(sessions))
}

pub(crate) async fn get_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PhotoSession>> {
    db::find_photo_session_by_id(&state.pool, user.user_id, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn upload_photo(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<SessionPhoto>)> {
    if !db::session_exists_for_user(&state.pool, user.user_id, session_id).await? {
        return Err(ApiError::NotFound);
    }

    let (angle, content_type, bytes) = read_image_part(multipart).await?;
    let byte_size = i64::try_from(bytes.len())?;
    let new = NewPhoto::new(angle, content_type, byte_size)
        .map_err(|e| ApiError::Validation { field: e.field() })?;

    let photo_id = Uuid::new_v4();
    let key = format!("{}/{}/{}", user.user_id.0, session_id, photo_id);
    state.store.put(&key, &bytes).await?; // bytes first — nothing to dangle on failure

    match db::insert_photo(&state.pool, session_id, photo_id, &new, &key).await {
        Ok(photo) => Ok((StatusCode::CREATED, Json(photo))),
        Err(e) => {
            let _ = state.store.delete(&key).await; // compensate the orphan object
            Err(e)
        }
    }
}

pub(crate) async fn download_photo(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((session_id, photo_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Response> {
    let location = db::find_photo_location(&state.pool, user.user_id, session_id, photo_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let bytes = state.store.get(&location.storage_key).await?; // store-miss → opaque 500
    Ok(([(header::CONTENT_TYPE, location.content_type)], bytes).into_response())
}

pub(crate) async fn delete_photo(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((session_id, photo_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let location = db::find_photo_location(&state.pool, user.user_id, session_id, photo_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    state.store.delete(&location.storage_key).await?; // bytes before the row
    db::delete_photo_row(&state.pool, photo_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn delete_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if !db::session_exists_for_user(&state.pool, user.user_id, session_id).await? {
        return Err(ApiError::NotFound);
    }
    for key in db::photo_keys_for_session(&state.pool, session_id).await? {
        let _ = state.store.delete(&key).await; // best-effort byte cleanup
    }
    db::delete_photo_session(&state.pool, user.user_id, session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Read the optional `angle` text part and the required image `file` part from a
/// multipart body, mapping every shape failure to the uniform `ApiError` (axum's
/// `MultipartError` must not escape — the `http::parse_body` precedent).
async fn read_image_part(
    mut multipart: Multipart,
) -> ApiResult<(Option<Angle>, ImageContentType, Bytes)> {
    let bad_file = || ApiError::Validation { field: "file" };

    let mut angle: Option<Angle> = None;
    let mut file: Option<(ImageContentType, Bytes)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| bad_file())? {
        match field.name() {
            Some("angle") => {
                let text = field.text().await.map_err(|_| bad_file())?;
                angle = Some(
                    Angle::parse(&text).map_err(|e| ApiError::Validation { field: e.field() })?,
                );
            }
            Some("file") => {
                let raw = field
                    .content_type()
                    .ok_or(ApiError::Validation {
                        field: "content_type",
                    })?
                    .to_owned();
                let content_type = ImageContentType::parse(&raw)
                    .map_err(|e| ApiError::Validation { field: e.field() })?;
                let bytes = field.bytes().await.map_err(|_| bad_file())?;
                file = Some((content_type, bytes));
            }
            _ => {
                let _ = field.bytes().await; // drain and ignore unknown parts
            }
        }
    }

    let (content_type, bytes) = file.ok_or_else(bad_file)?;
    Ok((angle, content_type, bytes))
}
