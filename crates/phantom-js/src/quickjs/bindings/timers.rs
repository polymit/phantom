//! Timer bindings for QuickJS.
//!
//! Provides setTimeout, clearTimeout, setInterval, clearInterval.
//! Timer callbacks are stored and executed within the JS context.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

static TIMER_COUNTER: AtomicU32 = AtomicU32::new(1);

/// The type of timer registered.
#[derive(Debug, Clone)]
pub enum TimerKind {
    /// One-shot timer (setTimeout).
    Timeout { delay_ms: u32 },
    /// Repeating timer (setInterval).
    Interval { interval_ms: u32 },
}

/// Timer state tracking for the JS runtime.
pub struct TimerRegistry {
    /// Active timer IDs and their kinds.
    pub active_timers: HashMap<u32, TimerKind>,
    /// Pending zero-delay callbacks to be executed in the next tick (Bug 4).
    pub pending_zero_delay: Vec<String>,
}

impl TimerRegistry {
    /// Create a new empty timer registry.
    pub fn new() -> Self {
        Self {
            active_timers: HashMap::new(),
            pending_zero_delay: Vec::new(),
        }
    }

    /// Register a timeout and return its timer ID.
    pub fn register(&mut self, callback_src: String, delay_ms: u32, repeating: bool) -> u32 {
        let id = TIMER_COUNTER.fetch_add(1, Ordering::SeqCst);

        if delay_ms == 0 {
            self.pending_zero_delay.push(callback_src);
        } else {
            let kind = if repeating {
                TimerKind::Interval {
                    interval_ms: delay_ms,
                }
            } else {
                TimerKind::Timeout { delay_ms }
            };
            self.active_timers.insert(id, kind);
        }
        id
    }

    /// Cancel a timer by ID.
    pub fn clear_timer(&mut self, id: u32) {
        self.active_timers.remove(&id);
    }

    /// Flush all pending zero-delay callbacks for execution (Bug 4).
    pub fn flush_zero_delay_timers(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_zero_delay)
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
