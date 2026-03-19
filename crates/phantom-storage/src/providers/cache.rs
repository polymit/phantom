use crate::errors::StorageError;
use sha2::{Digest, Sha256};
use sled::Db;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Cache storage tracking metadata in sled and storing payloads in filesystem blobs.
pub struct CacheStorage {
    meta_db: Db,
    blobs_dir: PathBuf,
}

impl CacheStorage {
    pub fn new(session_dir: &Path) -> Result<Self, StorageError> {
        let cache_dir = session_dir.join("cache");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let meta_db = sled::open(cache_dir.join("meta.sled"))?;
        let blobs_dir = cache_dir.join("blobs");
        if !blobs_dir.exists() {
            fs::create_dir_all(&blobs_dir)?;
        }

        Ok(Self { meta_db, blobs_dir })
    }

    /// Store a response blob and return its SHA-256 hash.
    pub fn put_blob(&self, data: &[u8]) -> Result<String, StorageError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize());
        let blob_path = self.blobs_dir.join(&hash);

        if !blob_path.exists() {
            // Atomic write: write to temp, then rename
            let temp_path = blob_path.with_extension("tmp");
            {
                let mut file = File::create(&temp_path)?;
                file.write_all(data)?;
            }
            fs::rename(temp_path, &blob_path)?;
        }

        Ok(hash)
    }

    /// Retrieve a blob by its hash.
    pub fn get_blob(&self, hash: &str) -> Result<Option<Vec<u8>>, StorageError> {
        if hash.contains("..") || hash.contains('/') || hash.contains('\\') {
            return Err(StorageError::PathTraversal(
                "Invalid hash format".to_string(),
            ));
        }

        let blob_path = self.blobs_dir.join(hash);
        if blob_path.exists() {
            let mut file = File::open(blob_path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Save metadata for a cache entry.
    pub fn save_metadata(
        &self,
        origin: &str,
        key_url: &str,
        meta_json: &str,
    ) -> Result<(), StorageError> {
        let db_key = format!("{}:{}", origin, key_url);
        self.meta_db
            .insert(db_key.as_bytes(), meta_json.as_bytes())?;
        self.meta_db.flush()?;
        Ok(())
    }

    /// Retrieve metadata for a cache entry.
    pub fn get_metadata(
        &self,
        origin: &str,
        key_url: &str,
    ) -> Result<Option<String>, StorageError> {
        let db_key = format!("{}:{}", origin, key_url);
        if let Some(ivec) = self.meta_db.get(db_key.as_bytes())? {
            Ok(String::from_utf8(ivec.to_vec()).ok())
        } else {
            Ok(None)
        }
    }
}
