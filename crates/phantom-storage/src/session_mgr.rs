use crate::errors::StorageError;
use crate::providers::cache::CacheStorage;
use crate::providers::cookie::CookieStorage;
use crate::providers::idb::IndexedDbStorage;
use crate::providers::local::LocalStorage;
use crate::quota_mgr::QuotaManager;
use crate::security::get_secure_session_dir;
use std::path::{Path, PathBuf};

/// Coordinates storage initialization inside an isolated namespace.
pub struct SessionStorageManager {
    pub session_id: String,
    pub session_dir: PathBuf,
    pub local: LocalStorage,
    pub cookie: CookieStorage,
    pub idb: IndexedDbStorage,
    pub cache: CacheStorage,
    pub quota: QuotaManager,
}

impl SessionStorageManager {
    #[tracing::instrument(skip(base_dir))]
    pub fn new(
        base_dir: &Path,
        session_id: &str,
        max_quota_bytes: usize,
    ) -> Result<Self, StorageError> {
        // Enforce strong isolation via security validation helper (D-15)
        let session_dir = get_secure_session_dir(base_dir, session_id)?;

        let local = LocalStorage::new(&session_dir)?;
        let cookie = CookieStorage::new(&session_dir)?;
        let idb = IndexedDbStorage::new(&session_dir)?;
        let cache = CacheStorage::new(&session_dir)?;
        let quota = QuotaManager::new(max_quota_bytes);

        Ok(Self {
            session_id: session_id.to_string(),
            session_dir,
            local,
            cookie,
            idb,
            cache,
            quota,
        })
    }
}
