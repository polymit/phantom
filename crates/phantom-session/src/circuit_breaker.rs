use parking_lot::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

struct Inner {
    state: CircuitState,
    failures: usize,
    last_failure_time: Option<Instant>,
    successes_in_half_open: usize,
}

/// A thread-safe Circuit Breaker for protecting resource acquisition.
pub struct CircuitBreaker {
    failure_threshold: usize,
    reset_timeout: Duration,
    half_open_max_calls: usize,
    inner: RwLock<Inner>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            half_open_max_calls: 3,
            inner: RwLock::new(Inner {
                state: CircuitState::Closed,
                failures: 0,
                last_failure_time: None,
                successes_in_half_open: 0,
            }),
        }
    }

    pub fn state(&self) -> CircuitState {
        let mut inner = self.inner.write();

        if inner.state == CircuitState::Open
            && inner
                .last_failure_time
                .is_some_and(|t| t.elapsed() > self.reset_timeout)
        {
            inner.state = CircuitState::HalfOpen;
        }

        inner.state
    }

    pub fn can_call(&self) -> bool {
        self.state() != CircuitState::Open
    }

    pub fn record_success(&self) {
        let mut inner = self.inner.write();
        inner.failures = 0;
        inner.last_failure_time = None;

        if inner.state == CircuitState::HalfOpen {
            inner.successes_in_half_open += 1;
            if inner.successes_in_half_open >= self.half_open_max_calls {
                inner.state = CircuitState::Closed;
                inner.successes_in_half_open = 0;
            }
        }
    }

    pub fn record_failure(&self) {
        let mut inner = self.inner.write();
        inner.failures += 1;
        inner.last_failure_time = Some(Instant::now());

        if inner.failures >= self.failure_threshold {
            inner.state = CircuitState::Open;
        }
    }
}
