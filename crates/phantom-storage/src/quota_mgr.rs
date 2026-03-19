use crate::errors::StorageError;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct QuotaManager {
    max_bytes: usize,
    used_bytes: AtomicUsize,
}

impl QuotaManager {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            used_bytes: AtomicUsize::new(0),
        }
    }

    pub fn add_usage(&self, bytes: usize, resource: &str) -> Result<(), StorageError> {
        let current = self.used_bytes.load(Ordering::Relaxed);
        if current + bytes > self.max_bytes {
            return Err(StorageError::QuotaExceeded {
                resource: resource.to_string(),
                used: current,
                limit: self.max_bytes,
            });
        }
        self.used_bytes.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    }

    pub fn free_usage(&self, bytes: usize) {
        let current = self.used_bytes.load(Ordering::Relaxed);
        if bytes > current {
            self.used_bytes.store(0, Ordering::Relaxed);
        } else {
            self.used_bytes.fetch_sub(bytes, Ordering::Relaxed);
        }
    }

    pub fn get_usage(&self) -> usize {
        self.used_bytes.load(Ordering::Relaxed)
    }
}
