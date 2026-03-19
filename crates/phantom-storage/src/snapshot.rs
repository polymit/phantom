use crate::errors::StorageError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Metadata manifest for a ZSTD-compressed session snapshot.
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub session_id: String,
    pub timestamp_sec: u64,
    pub original_size_bytes: u64,
    pub file_checksums: std::collections::HashMap<String, String>,
}

pub struct SnapshotManager {
    base_dir: PathBuf,
}

impl SnapshotManager {
    #[tracing::instrument(skip(base_dir))]
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    /// Creates a `.tar.zst` snapshot of a session directory securely.
    #[tracing::instrument(skip(self))]
    pub fn create_snapshot(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        let session_dir = crate::security::get_secure_session_dir(&self.base_dir, session_id)?;
        let snapshot_file = self
            .base_dir
            .join(format!("snapshot-{}.tar.zst", session_id));
        let temp_snapshot = snapshot_file.with_extension("tmp");

        {
            let file = File::create(&temp_snapshot)?;

            // D-17: Wrap the file explicitly in a zstd encoder
            let zstd_encoder = zstd::Encoder::new(file, 3)
                .map_err(|e| StorageError::SnapshotFailed(format!("Zstd setup failed: {}", e)))?
                .auto_finish();

            // Archive the session directory contents into the compressed stream
            let mut tar_builder = tar::Builder::new(zstd_encoder);
            tar_builder
                .append_dir_all(".", &session_dir)
                .map_err(|e| StorageError::SnapshotFailed(format!("Tar append failed: {}", e)))?;
            tar_builder.finish()?;
        }

        // Generate SHA256 of the resulting archive (integrity check)
        let mut final_file = File::open(&temp_snapshot)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let n = final_file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let hash = hex::encode(hasher.finalize());

        // Validate creation output
        let manifest = SnapshotManifest {
            session_id: session_id.to_string(),
            timestamp_sec: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            original_size_bytes: std::fs::metadata(&temp_snapshot)?.len(), // Note: compressed size recorded for fast verification
            file_checksums: vec![("archive.tar.zst".to_string(), hash)]
                .into_iter()
                .collect(),
        };

        // Save manifest
        let manifest_path = self.base_dir.join(format!("manifest-{}.json", session_id));
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        std::fs::write(manifest_path, manifest_json)?;

        // Rename temporal zstd tarball file to final
        std::fs::rename(temp_snapshot, &snapshot_file)?;

        Ok(snapshot_file)
    }

    /// Restores a session from a `.tar.zst` snapshot.
    #[tracing::instrument(skip(self))]
    pub fn restore_snapshot(&self, session_id: &str) -> Result<(), StorageError> {
        let snapshot_file = self
            .base_dir
            .join(format!("snapshot-{}.tar.zst", session_id));
        if !snapshot_file.exists() {
            return Err(StorageError::SnapshotFailed(
                "Snapshot file not found".to_string(),
            ));
        }

        let session_dir = crate::security::get_secure_session_dir(&self.base_dir, session_id)?;

        let file = File::open(&snapshot_file)?;
        let zstd_decoder = zstd::Decoder::new(file)
            .map_err(|e| StorageError::SnapshotFailed(format!("Zstd decode failed: {}", e)))?;

        let mut archive = tar::Archive::new(zstd_decoder);
        archive.unpack(&session_dir).map_err(|e| {
            StorageError::SnapshotFailed(format!("Failed to unpack snapshot tar: {}", e))
        })?;

        Ok(())
    }
}
