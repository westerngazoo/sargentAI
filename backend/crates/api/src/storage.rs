//! The object-store seam (SPEC-0006 §2.2/§3.2): photo bytes live behind a
//! `put`/`get`/`delete` trait so they stay out of Postgres and out of the cloud
//! during dev/CI. `LocalObjectStore` is the filesystem implementation used now;
//! the real S3-compatible implementation is a drop-in at R-0026.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObjectStoreError {
    /// No object exists at the key (an S3 `GetObject` 404 maps here).
    #[error("object not found")]
    Missing,
    /// The key is not traversal-safe (contains `.`/`..`/an empty segment).
    #[error("invalid object key")]
    InvalidKey,
    #[error("object store io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A content-addressed byte store. The contract — `delete` is **idempotent**
/// (a missing key is `Ok`) and `get` on a missing key is
/// [`ObjectStoreError::Missing`] — binds every implementation, including the
/// R-0026 S3 impl.
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Store `bytes` at `key`, overwriting any existing object.
    ///
    /// # Errors
    /// [`ObjectStoreError::InvalidKey`] for a traversal-unsafe key; IO errors.
    async fn put(&self, key: &str, bytes: &Bytes) -> Result<(), ObjectStoreError>;

    /// Read the object at `key`.
    ///
    /// # Errors
    /// [`ObjectStoreError::Missing`] when absent;
    /// [`ObjectStoreError::InvalidKey`] for a traversal-unsafe key.
    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError>;

    /// Delete the object at `key`. Idempotent — a missing key is `Ok`.
    ///
    /// # Errors
    /// [`ObjectStoreError::InvalidKey`] for a traversal-unsafe key; IO errors.
    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError>;
}

/// Stores objects as files under `root`, one file per key. Keys are
/// `{user}/{session}/{photo}` (UUID-only) so the nested layout is a clean
/// directory tree; any `.`/`..`/empty segment is rejected (defense in depth).
pub struct LocalObjectStore {
    root: PathBuf,
}

impl LocalObjectStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn resolve(&self, key: &str) -> Result<PathBuf, ObjectStoreError> {
        if key
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return Err(ObjectStoreError::InvalidKey);
        }
        Ok(self.root.join(key))
    }
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put(&self, key: &str, bytes: &Bytes) -> Result<(), ObjectStoreError> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, bytes.as_ref()).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let path = self.resolve(key)?;
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Bytes::from(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ObjectStoreError::Missing),
            Err(e) => Err(ObjectStoreError::Io(e)),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let path = self.resolve(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ObjectStoreError::Io(e)),
        }
    }
}
