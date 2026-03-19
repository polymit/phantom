use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::SystemTime;
use uuid::Uuid;

/// An entry in the cooperative scheduler's run queue.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RunQueueEntry {
    pub session_id: Uuid,
    pub priority: u32,
    pub last_run: SystemTime,
}

impl Ord for RunQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        let prio_cmp = self.priority.cmp(&other.priority);
        if prio_cmp != Ordering::Equal {
            return prio_cmp;
        }
        // If priority is equal, older last_run (smaller timestamp) comes first
        // Note: reverse to make earlier times greater in BinaryHeap
        other.last_run.cmp(&self.last_run)
    }
}

impl PartialOrd for RunQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A cooperative scheduler prioritizing sessions needing execution.
pub struct Scheduler {
    run_queue: BinaryHeap<RunQueueEntry>,
}

impl Scheduler {
    /// Create a new session scheduler.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            run_queue: BinaryHeap::new(),
        }
    }

    /// Enqueue a session for execution with a specific base priority.
    #[tracing::instrument(skip(self))]
    pub fn enqueue(&mut self, session_id: Uuid, priority: u32) {
        self.run_queue.push(RunQueueEntry {
            session_id,
            priority,
            last_run: SystemTime::now(),
        });
    }

    /// Get the next session ready for execution.
    #[tracing::instrument(skip(self))]
    pub fn next_session(&mut self) -> Option<Uuid> {
        self.run_queue.pop().map(|entry| entry.session_id)
    }

    /// Re-enqueue a session with lowered priority (e.g., after it yields or times out).
    #[tracing::instrument(skip(self))]
    pub fn deprioritize(&mut self, session_id: Uuid, current_priority: u32) {
        let new_priority = current_priority.saturating_sub(1);
        self.enqueue(session_id, new_priority);
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
