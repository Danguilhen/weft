// SPDX-License-Identifier: AGPL-3.0-or-later

//! Content-addressed blob storage.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Uuid;
use weft_core::{hash_bytes, BlobHash};

/// Storage operations needed by the server and the future replication worker.
pub trait BlobStore: Send + Sync {
    fn path_for(&self, hash: &BlobHash) -> Result<PathBuf, StorageError>;
    fn put(&self, hash: &BlobHash, bytes: &[u8]) -> Result<(), StorageError>;
    fn get(&self, hash: &BlobHash) -> Result<Vec<u8>, StorageError>;
    fn exists(&self, hash: &BlobHash) -> Result<bool, StorageError>;
    fn delete(&self, hash: &BlobHash) -> Result<(), StorageError>;
    fn check(&self) -> Result<(), StorageError>;
}

/// Local filesystem backend using `blobs/sha256/ab/cd/<hash>`.
#[derive(Debug, Clone)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(root.join("blobs/sha256"))?;
        fs::create_dir_all(root.join("tmp"))?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl BlobStore for LocalStore {
    fn path_for(&self, hash: &BlobHash) -> Result<PathBuf, StorageError> {
        let value = hash.as_str();
        Ok(self
            .root
            .join("blobs/sha256")
            .join(&value[..2])
            .join(&value[2..4])
            .join(value))
    }

    fn put(&self, hash: &BlobHash, bytes: &[u8]) -> Result<(), StorageError> {
        let actual = hash_bytes(bytes);
        if actual != *hash {
            return Err(StorageError::HashMismatch {
                expected: hash.to_string(),
                actual: actual.to_string(),
            });
        }

        let path = self.path_for(hash)?;
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp = self
            .root
            .join("tmp")
            .join(format!("{}.part", Uuid::new_v4()));
        let result = (|| -> Result<(), StorageError> {
            let mut file = File::create(&temp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &path)?;
            if let Some(parent) = path.parent() {
                if let Ok(directory) = File::open(parent) {
                    let _ = directory.sync_all();
                }
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    fn get(&self, hash: &BlobHash) -> Result<Vec<u8>, StorageError> {
        let path = self.path_for(hash)?;
        let mut file = File::open(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(hash.to_string())
            } else {
                StorageError::Io(error)
            }
        })?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let actual = hash_bytes(&bytes);
        if actual != *hash {
            return Err(StorageError::HashMismatch {
                expected: hash.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(bytes)
    }

    fn exists(&self, hash: &BlobHash) -> Result<bool, StorageError> {
        Ok(self.path_for(hash)?.is_file())
    }

    fn delete(&self, hash: &BlobHash) -> Result<(), StorageError> {
        let path = self.path_for(hash)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    fn check(&self) -> Result<(), StorageError> {
        let path = self
            .root
            .join("tmp")
            .join(format!("{}.health", Uuid::new_v4()));
        fs::write(&path, b"ok")?;
        fs::remove_file(path)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("blob {0} was not found")]
    NotFound(String),
    #[error("blob hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("invalid content hash")]
    InvalidHash,
}

#[cfg(test)]
mod tests {
    use super::{BlobStore, LocalStore};
    use weft_core::hash_bytes;

    #[test]
    fn puts_and_reads_content_addressed_blobs() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::new(directory.path()).unwrap();
        let bytes = b"GIF89a test";
        let hash = hash_bytes(bytes);

        store.put(&hash, bytes).unwrap();
        assert!(store.exists(&hash).unwrap());
        assert_eq!(store.get(&hash).unwrap(), bytes);

        let path = store.path_for(&hash).unwrap();
        assert!(path.ends_with(hash.as_str()));
        assert!(path.to_string_lossy().contains("/blobs/sha256/"));
    }

    #[test]
    fn rejects_a_wrong_hash() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::new(directory.path()).unwrap();
        let hash = hash_bytes(b"expected");
        assert!(store.put(&hash, b"different").is_err());
        assert!(!store.exists(&hash).unwrap());
    }

    #[test]
    fn duplicate_put_is_idempotent_and_delete_is_safe() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::new(directory.path()).unwrap();
        let bytes = b"same bytes";
        let hash = hash_bytes(bytes);
        store.put(&hash, bytes).unwrap();
        store.put(&hash, bytes).unwrap();
        store.delete(&hash).unwrap();
        assert!(!store.exists(&hash).unwrap());
        store.delete(&hash).unwrap();
    }

    #[test]
    fn check_writes_to_a_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::new(directory.path()).unwrap();
        store.check().unwrap();
    }
}
