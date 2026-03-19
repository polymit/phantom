use crate::errors::StorageError;
use sled::Db;
use std::path::Path;

/// LocalStorage provider scoped strictly to the session directory via sled.
pub struct LocalStorage {
    db: Db,
}

impl LocalStorage {
    pub fn new(session_dir: &Path) -> Result<Self, StorageError> {
        let db_path = session_dir.join("localstorage.sled");
        let db = sled::open(db_path)?;
        Ok(Self { db })
    }

    pub fn set(&self, origin: &str, key: &str, value: &str) -> Result<(), StorageError> {
        let prefixed_key = format!("{}:{}", origin, key);
        self.db.insert(prefixed_key.as_bytes(), value.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get(&self, origin: &str, key: &str) -> Result<Option<String>, StorageError> {
        let prefixed_key = format!("{}:{}", origin, key);
        if let Some(ivec) = self.db.get(prefixed_key.as_bytes())? {
            Ok(String::from_utf8(ivec.to_vec()).ok())
        } else {
            Ok(None)
        }
    }

    pub fn remove(&self, origin: &str, key: &str) -> Result<bool, StorageError> {
        let prefixed_key = format!("{}:{}", origin, key);
        let existed = self.db.remove(prefixed_key.as_bytes())?.is_some();
        if existed {
            self.db.flush()?;
        }
        Ok(existed)
    }

    pub fn clear_origin(&self, origin: &str) -> Result<(), StorageError> {
        let prefix = format!("{}:", origin);
        for (k, _) in self.db.scan_prefix(prefix.as_bytes()).flatten() {
            self.db.remove(k)?;
        }
        self.db.flush()?;
        Ok(())
    }
}
