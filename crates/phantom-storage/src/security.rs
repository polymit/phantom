use crate::errors::StorageError;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Validates a session UUID string and strictly enforces path containment.
/// Creates the directory if it doesn't exist with 0700 permissions.
/// Returns the canonicalized path.
pub fn get_secure_session_dir(base_dir: &Path, session_id: &str) -> Result<PathBuf, StorageError> {
    // 1. Validate UUID format
    if Uuid::parse_str(session_id).is_err() {
        return Err(StorageError::InvalidSessionId(session_id.to_string()));
    }

    // 2. Reject any attempt at path traversal tokens
    if session_id.contains("..") || session_id.contains('/') || session_id.contains('\\') {
        return Err(StorageError::PathTraversal(session_id.to_string()));
    }

    // Ensure base directory exists
    if !base_dir.exists() {
        fs::create_dir_all(base_dir)?;
        let mut base_perms = fs::metadata(base_dir)?.permissions();
        base_perms.set_mode(0o700);
        fs::set_permissions(base_dir, base_perms)?;
    }

    let session_dir = base_dir.join(session_id);

    // 3. Create directory if not exists
    if !session_dir.exists() {
        fs::create_dir_all(&session_dir)?;
    }

    // 4. Set 0700 permissions
    let mut perms = fs::metadata(&session_dir)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(&session_dir, perms)?;

    // 5. Canonicalize (resolves symlinks, normalizes path)
    let canonical_path = session_dir.canonicalize()?;
    let canonical_base = base_dir.canonicalize()?;

    // 6. Ensure the canonicalized session path actually starts with the base path
    if !canonical_path.starts_with(&canonical_base) {
        return Err(StorageError::PathTraversal(
            "Canonical path escaped base directory".to_string(),
        ));
    }

    Ok(canonical_path)
}
