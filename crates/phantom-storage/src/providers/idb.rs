use crate::errors::StorageError;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// IndexedDB provider backed by SQLite per origin.
pub struct IndexedDbStorage {
    storage_dir: PathBuf,
}

impl IndexedDbStorage {
    pub fn new(session_dir: &Path) -> Result<Self, StorageError> {
        let storage_dir = session_dir.join("indexeddb");
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }
        Ok(Self { storage_dir })
    }

    /// Gets or creates a SQLite connection for a specific origin.
    pub fn get_connection(&self, origin_hash: &str) -> Result<Connection, StorageError> {
        // Enforce strict origin hashing in real usage, here we just use the hash string
        if origin_hash.contains("..") || origin_hash.contains('/') || origin_hash.contains('\\') {
            return Err(StorageError::PathTraversal(
                "Invalid origin hash".to_string(),
            ));
        }

        let db_path = self.storage_dir.join(format!("{}.sqlite", origin_hash));
        let conn = Connection::open(&db_path)?;

        // D-16: Enforce WAL mode for concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        Ok(conn)
    }

    /// Removes an entire database for an origin.
    pub fn delete_database(&self, origin_hash: &str) -> Result<bool, StorageError> {
        if origin_hash.contains("..") || origin_hash.contains('/') {
            return Err(StorageError::PathTraversal(
                "Invalid origin hash".to_string(),
            ));
        }

        let db_path = self.storage_dir.join(format!("{}.sqlite", origin_hash));
        if db_path.exists() {
            fs::remove_file(&db_path)?;
            // Also attempt to remove WAL and SHM files if they exist
            let _ = fs::remove_file(db_path.with_extension("sqlite-wal"));
            let _ = fs::remove_file(db_path.with_extension("sqlite-shm"));
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
