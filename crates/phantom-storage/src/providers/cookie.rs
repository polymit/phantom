use crate::errors::StorageError;
use cookie_store::CookieStore;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// CookieStorage provider storing cookies isolated within the session directory.
pub struct CookieStorage {
    store: CookieStore,
    storage_path: PathBuf,
}

impl CookieStorage {
    pub fn new(session_dir: &Path) -> Result<Self, StorageError> {
        let storage_path = session_dir.join("cookies.json");

        let store = if storage_path.exists() {
            let file = File::open(&storage_path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader::<_, cookie_store::CookieStore>(reader)
                .unwrap_or_else(|_| cookie_store::CookieStore::default())
        } else {
            CookieStore::default()
        };

        Ok(Self {
            store,
            storage_path,
        })
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        // Atomic write via temp file per architecture rules
        let temp_path = self.storage_path.with_extension("tmp");
        {
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &self.store).map_err(|e| {
                StorageError::InitializationFailed(format!("Failed to save cookies: {}", e))
            })?;
        }
        fs::rename(temp_path, &self.storage_path)?;
        Ok(())
    }

    pub fn get_store(&self) -> &CookieStore {
        &self.store
    }

    pub fn get_store_mut(&mut self) -> &mut CookieStore {
        &mut self.store
    }
}
