use crate::pool::IsolatePool;
use crate::scheduler::Scheduler;
use crate::types::{EngineKind, Session, SessionState};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Orchestrates Phantom Engine sessions, handling the lifecycle, pooling, and scheduling.
/// Follows D-10: V8 snapshots for cloning, not OS forks.
pub struct SessionBroker {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    pub isolate_pool: Arc<IsolatePool>,
    pub scheduler: Arc<RwLock<Scheduler>>,
}

impl SessionBroker {
    /// Initialize a new Session Broker.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            isolate_pool: Arc::new(IsolatePool::new()),
            scheduler: Arc::new(RwLock::new(Scheduler::new())),
        }
    }

    /// Create a new session of the requested engine tier.
    #[tracing::instrument(skip(self))]
    pub fn create_session(&self, kind: EngineKind) -> Uuid {
        let session = Session::new(kind);
        let id = session.id;

        // Enqueue into scheduler
        self.scheduler.write().enqueue(id, 10); // Base priority 10

        let mut map = self.sessions.write();
        map.insert(id, session);
        id
    }

    /// Retrieve a session for mutation or execution.
    #[tracing::instrument(skip(self, f))]
    pub fn get_session<F, R>(&self, id: Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut map = self.sessions.write();
        if let Some(session) = map.get_mut(&id) {
            session.touch();
            Some(f(session))
        } else {
            None
        }
    }

    /// Suspend a session, persisting it (stubbed for now) and releasing resources.
    #[tracing::instrument(skip(self))]
    pub fn suspend_session(&self, id: Uuid) -> bool {
        let mut map = self.sessions.write();
        if let Some(session) = map.get_mut(&id)
            && session.state == SessionState::Active
        {
            session.mark_suspended();
            return true;
        }
        false
    }

    /// Resume a suspended session back to active state.
    #[tracing::instrument(skip(self))]
    pub fn resume_session(&self, id: Uuid) -> bool {
        let mut map = self.sessions.write();
        if let Some(session) = map.get_mut(&id)
            && session.state == SessionState::Suspended
        {
            session.mark_active();
            // Re-enqueue into scheduler
            self.scheduler.write().enqueue(id, 10);
            return true;
        }
        false
    }

    /// Clone an existing session via V8 snapshot mechanics per D-10.
    #[tracing::instrument(skip(self))]
    pub fn clone_session(&self, parent_id: Uuid) -> Option<Uuid> {
        let kind = {
            let map = self.sessions.read();
            let parent = map.get(&parent_id)?;
            parent.engine_kind
        };

        // Decision D-10: V8 snapshots for session cloning, NOT OS forks.
        // 1. Serialize V8 isolate to snapshot blob.
        // 2. Deserialize snapshot blob into the new isolate.
        // Since we are mocking the engine internals here, we explicitly create
        // a new session to represent the freshly deserialized snapshot.
        let child_id = self.create_session(kind);
        Some(child_id)
    }

    /// Destroy a session permanently.
    #[tracing::instrument(skip(self))]
    pub fn destroy_session(&self, id: Uuid) -> bool {
        // We could just remove it, but marking as destroyed helps with auditing
        let mut map = self.sessions.write();
        if let Some(session) = map.get_mut(&id) {
            session.mark_destroyed();
            // Actually remove it to free memory
        }
        map.remove(&id).is_some()
    }

    /// Return the count of currently active sessions.
    pub fn active_count(&self) -> usize {
        let map = self.sessions.read();
        map.values()
            .filter(|s| s.state == SessionState::Active)
            .count()
    }

    /// Return the count of suspended sessions.
    pub fn suspended_count(&self) -> usize {
        let map = self.sessions.read();
        map.values()
            .filter(|s| s.state == SessionState::Suspended)
            .count()
    }
}

impl Default for SessionBroker {
    fn default() -> Self {
        Self::new()
    }
}
