//! Unit tests for the `fitai_api::storage` object-store seam — the
//! `ObjectStore` trait contract and its `LocalObjectStore` filesystem impl
//! (SPEC-0006 §2.2, §3.2).
//!
//! Authored by the qa agent during R-0006 step 3 (test planning), BEFORE the
//! `api::storage` module exists. Pre-implementation red state = compile failure
//! (the module / types are absent). Implementation step 5 makes these green.
//!
//! These are plain `#[tokio::test]`s — no Postgres, no router. They pin the
//! trait CONTRACT that the R-0026 S3 impl must also honour (SPEC-0006 §3.2):
//! - put → get round-trips the exact bytes;
//! - get on a missing key → `ObjectStoreError::Missing`;
//! - delete is idempotent — deleting a missing key → `Ok`;
//! - a key containing `..` is rejected (path-traversal defense in depth, AC9).
//!
//! Coverage: SAC8 → AC8 (the local store + the missing-key / `..`-rejection
//! paths) and the SAC9 traversal-safety guard.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote keys and the `..` token as prose.
#![allow(clippy::doc_markdown)]

use bytes::Bytes;
use fitai_api::storage::{LocalObjectStore, ObjectStore, ObjectStoreError};
use tempfile::TempDir;

/// A store rooted in a fresh temp dir, returned with the owning `TempDir` so the
/// directory survives for the test's duration (dropping it removes the files).
fn store() -> (LocalObjectStore, TempDir) {
    let dir = tempfile::tempdir().expect("a temp dir must be creatable");
    (LocalObjectStore::new(dir.path()), dir)
}

/// A UUID-shaped key, exactly the `{user}/{session}/{photo}` scheme the upload
/// handler writes under (SPEC-0006 §2.1).
fn sample_key() -> String {
    format!(
        "{}/{}/{}",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4()
    )
}

#[tokio::test]
async fn put_then_get_round_trips_the_exact_bytes() {
    let (store, _dir) = store();
    let key = sample_key();
    let payload = Bytes::from_static(b"\x89PNG\r\n\x1a\n the exact bytes, verbatim");

    store.put(&key, &payload).await.expect("put must succeed");
    let read = store.get(&key).await.expect("get must succeed after put");

    assert_eq!(
        read, payload,
        "get must return the exact bytes that were put"
    );
}

#[tokio::test]
async fn put_overwrites_an_existing_key() {
    let (store, _dir) = store();
    let key = sample_key();

    store
        .put(&key, &Bytes::from_static(b"first"))
        .await
        .unwrap();
    store
        .put(&key, &Bytes::from_static(b"second"))
        .await
        .unwrap();

    let read = store.get(&key).await.unwrap();
    assert_eq!(read, Bytes::from_static(b"second"));
}

#[tokio::test]
async fn get_on_a_missing_key_is_missing() {
    let (store, _dir) = store();
    let err = store
        .get(&sample_key())
        .await
        .expect_err("get on a never-written key must error");
    assert!(
        matches!(err, ObjectStoreError::Missing),
        "a missing key must map to ObjectStoreError::Missing, got {err:?}"
    );
}

#[tokio::test]
async fn delete_then_get_is_missing() {
    let (store, _dir) = store();
    let key = sample_key();
    store
        .put(&key, &Bytes::from_static(b"bytes"))
        .await
        .unwrap();

    store.delete(&key).await.expect("delete of a present key");

    let err = store
        .get(&key)
        .await
        .expect_err("the bytes must be gone after delete");
    assert!(matches!(err, ObjectStoreError::Missing));
}

#[tokio::test]
async fn delete_is_idempotent_for_a_missing_key() {
    let (store, _dir) = store();
    // Per the trait contract (SPEC-0006 §3.2), deleting a never-written key is
    // a no-op success, not an error.
    store
        .delete(&sample_key())
        .await
        .expect("delete of a missing key must be Ok (idempotent)");
}

#[tokio::test]
async fn delete_twice_is_ok() {
    let (store, _dir) = store();
    let key = sample_key();
    store
        .put(&key, &Bytes::from_static(b"bytes"))
        .await
        .unwrap();

    store.delete(&key).await.expect("first delete");
    store
        .delete(&key)
        .await
        .expect("second delete must also be Ok (idempotent)");
}

#[tokio::test]
async fn a_key_containing_dot_dot_is_rejected_on_put() {
    let (store, _dir) = store();
    let err = store
        .put("../escape", &Bytes::from_static(b"x"))
        .await
        .expect_err("a key with `..` must be rejected (traversal defense)");
    // The exact variant is the impl's choice, but it must NOT be a silent
    // success — a `..` key must never reach the filesystem.
    assert!(
        !matches!(err, ObjectStoreError::Missing),
        "a `..` key on put must be rejected, not treated as a missing key"
    );
}

#[tokio::test]
async fn a_key_containing_dot_dot_is_rejected_on_get() {
    let (store, _dir) = store();
    let result = store.get("a/../../etc/passwd").await;
    assert!(
        result.is_err(),
        "a `..` key on get must be rejected, never resolved outside the root"
    );
}

#[tokio::test]
async fn a_nested_key_creates_intermediate_directories() {
    // The upload key is `{user}/{session}/{photo}` — two directory levels deep.
    // put must create them; this is the real shape, not a flat key.
    let (store, _dir) = store();
    let key = sample_key();
    store
        .put(&key, &Bytes::from_static(b"nested"))
        .await
        .expect("put must create intermediate dirs for a nested key");
    assert_eq!(
        store.get(&key).await.unwrap(),
        Bytes::from_static(b"nested")
    );
}
