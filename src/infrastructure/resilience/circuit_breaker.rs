use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// A `Closed -> Open -> HalfOpen -> Closed` circuit breaker for guarding calls
/// to a single transient-failure-prone dependency (e.g. a Redis connection).
///
/// `Open` rejects calls immediately until `reset_timeout_ms` elapses, at which
/// point a single probe call is allowed through (`HalfOpen`). That probe's
/// outcome decides whether the breaker closes again or re-opens.
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout_ms: u64,
    consecutive_failures: AtomicU32,
    state: AtomicU32,
    opened_at_ms: AtomicU64,
    probe_in_flight: AtomicU32,
}

const STATE_CLOSED: u32 = 0;
const STATE_OPEN: u32 = 1;
const STATE_HALF_OPEN: u32 = 2;

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            reset_timeout_ms,
            consecutive_failures: AtomicU32::new(0),
            state: AtomicU32::new(STATE_CLOSED),
            opened_at_ms: AtomicU64::new(0),
            probe_in_flight: AtomicU32::new(0),
        }
    }

    pub fn state(&self) -> CircuitState {
        match self.state.load(Ordering::SeqCst) {
            STATE_OPEN if self.reset_deadline_elapsed() => CircuitState::HalfOpen,
            STATE_OPEN => CircuitState::Open,
            STATE_HALF_OPEN => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    /// Returns `true` if a call is currently allowed to proceed. In
    /// `HalfOpen`, only a single probe is let through at a time.
    pub fn allow_request(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => self
                .probe_in_flight
                .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok(),
        }
    }

    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.state.store(STATE_CLOSED, Ordering::SeqCst);
        self.probe_in_flight.store(0, Ordering::SeqCst);
    }

    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= self.failure_threshold {
            self.state.store(STATE_OPEN, Ordering::SeqCst);
            self.opened_at_ms.store(now_ms(), Ordering::SeqCst);
            self.probe_in_flight.store(0, Ordering::SeqCst);
        }
    }

    fn reset_deadline_elapsed(&self) -> bool {
        let opened_at = self.opened_at_ms.load(Ordering::SeqCst);
        now_ms().saturating_sub(opened_at) >= self.reset_timeout_ms
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_closed_and_allows_requests() {
        let breaker = CircuitBreaker::new(3, 1000);
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allow_request());
    }

    #[test]
    fn opens_after_reaching_failure_threshold() {
        let breaker = CircuitBreaker::new(3, 60_000);
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.allow_request());
    }

    #[test]
    fn a_success_resets_the_failure_count() {
        let breaker = CircuitBreaker::new(3, 60_000);
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_success();
        breaker.record_failure();
        breaker.record_failure();

        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn transitions_to_half_open_once_the_reset_timeout_elapses() {
        let breaker = CircuitBreaker::new(1, 0);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        assert!(breaker.allow_request());
    }

    #[test]
    fn a_failed_probe_in_half_open_reopens_the_circuit() {
        let breaker = CircuitBreaker::new(1, 60_000);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        breaker.opened_at_ms.store(0, Ordering::SeqCst);
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn a_successful_probe_in_half_open_closes_the_circuit() {
        let breaker = CircuitBreaker::new(1, 0);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}
