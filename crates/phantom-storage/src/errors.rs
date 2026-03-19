use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path traversal attempt detected: {0}")]
    PathTraversal(String),

    #[error("Invalid session ID format: {0}")]
    InvalidSessionId(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Key-value store error: {0}")]
    Sled(#[from] sled::Error),

    #[error("Quota exceeded: {resource} used {used}/{limit} bytes")]
    QuotaExceeded {
        resource: String,
        used: usize,
        limit: usize,
    },

    #[error("Storage initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Snapshot error: {0}")]
    SnapshotFailed(String),
}
