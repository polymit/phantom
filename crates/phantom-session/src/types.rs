use std::time::SystemTime;
use uuid::Uuid;

/// Represents the JavaScript engine tier used for the session (D-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Tier 1: Fast startup, low memory. Used for 80% of sessions.
    QuickJS,
    /// Tier 2: Full ES2024 compliance. Slower startup, higher memory.
    V8,
}

/// The current state of a browser session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Actively processing or ready to process commands.
    Active,
    /// Suspended to disk/memory. Can be resumed.
    Suspended,
    /// Permanently destroyed.
    Destroyed,
}

/// Handle to an active isolate in the Isolates pool.
#[derive(Debug)]
pub struct IsolateHandle {
    pub id: Uuid,
    pub kind: EngineKind,
    pub created_at: SystemTime,
}

/// Resource constraints applied to a single session.
#[derive(Debug, Clone, Copy)]
pub struct ResourceBudget {
    /// Maximum allowed memory footprint in bytes.
    pub max_memory_bytes: usize,
    /// Maximum allowed execution time for a single sync event loop tick.
    pub max_execution_time_ms: u64,
    /// Maximum allowed concurrent tasks.
    pub max_concurrent_tasks: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB default
            max_execution_time_ms: 10_000,       // 10 seconds execution limit
            max_concurrent_tasks: 4,
        }
    }
}

/// A complete Phantom browser session representing an agent's context.
#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub state: SessionState,
    pub engine_kind: EngineKind,
    pub budget: ResourceBudget,
    pub created_at: SystemTime,
    pub last_accessed_at: SystemTime,
}

impl Session {
    pub fn new(engine_kind: EngineKind) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            state: SessionState::Active,
            engine_kind,
            budget: ResourceBudget::default(),
            created_at: now,
            last_accessed_at: now,
        }
    }

    /// Mark the session as accessed to prevent eviction
    pub fn touch(&mut self) {
        self.last_accessed_at = SystemTime::now();
    }

    pub fn mark_suspended(&mut self) {
        self.state = SessionState::Suspended;
    }

    pub fn mark_active(&mut self) {
        self.state = SessionState::Active;
    }

    pub fn mark_destroyed(&mut self) {
        self.state = SessionState::Destroyed;
    }
}
