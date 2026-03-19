use crate::circuit_breaker::CircuitBreaker;
use crate::types::{EngineKind, IsolateHandle};
use crossbeam::queue::SegQueue;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Lock-free pool of pre-warmed JS Isolates to ensure sub-10ms startup times.
pub struct IsolatePool {
    quickjs_pool: Arc<SegQueue<IsolateHandle>>,
    v8_pool: Arc<SegQueue<IsolateHandle>>,
    /// Circuit breaker to protect the engine when pool is exhausted or failing.
    pub circuit_breaker: CircuitBreaker,
}

impl IsolatePool {
    /// Create a new empty isolate pool.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            quickjs_pool: Arc::new(SegQueue::new()),
            v8_pool: Arc::new(SegQueue::new()),
            // default: open after 5 failures, reset after 30s
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(30)),
        }
    }

    /// Pre-warm the pool with a specific number of unused isolates.
    #[tracing::instrument(skip(self))]
    pub fn prewarm(&self, quickjs_count: usize, v8_count: usize) {
        for _ in 0..quickjs_count {
            self.quickjs_pool.push(IsolateHandle {
                id: Uuid::new_v4(),
                kind: EngineKind::QuickJS,
                created_at: SystemTime::now(),
            });
        }

        for _ in 0..v8_count {
            self.v8_pool.push(IsolateHandle {
                id: Uuid::new_v4(),
                kind: EngineKind::V8,
                created_at: SystemTime::now(),
            });
        }
    }

    /// Acquire an isolate from the pool of the requested engine kind.
    /// If the pool is empty, it returns `None` (caller should spawn a new one).
    #[tracing::instrument(skip(self))]
    pub fn acquire(&self, kind: EngineKind) -> Option<IsolateHandle> {
        if !self.circuit_breaker.can_call() {
            tracing::warn!("Isolate pool circuit breaker is OPEN");
            return None;
        }

        match kind {
            EngineKind::QuickJS => self.quickjs_pool.pop(),
            EngineKind::V8 => self.v8_pool.pop(),
        }
    }

    /// Return an isolate back to the pool after the session is suspended or destroyed.
    /// Only clean/scrubbed isolates should be returned in a full implementation.
    #[tracing::instrument(skip(self))]
    pub fn release(&self, handle: IsolateHandle) {
        match handle.kind {
            EngineKind::QuickJS => self.quickjs_pool.push(handle),
            EngineKind::V8 => self.v8_pool.push(handle),
        }
    }

    /// Get the number of available isolates for a given engine kind.
    pub fn available(&self, kind: EngineKind) -> usize {
        match kind {
            EngineKind::QuickJS => self.quickjs_pool.len(),
            EngineKind::V8 => self.v8_pool.len(),
        }
    }
}

impl Default for IsolatePool {
    fn default() -> Self {
        Self::new()
    }
}
