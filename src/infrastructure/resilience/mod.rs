pub mod circuit_breaker;
pub mod retry;

pub use circuit_breaker::CircuitBreaker;
pub use retry::{retry_with_backoff, RetryPolicy};

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, Copy)]
pub struct ResilienceConfig {
    pub failure_threshold: u32,
    pub reset_timeout_ms: u64,
    pub retry_max_attempts: u32,
    pub retry_initial_backoff_ms: u64,
    pub retry_backoff_multiplier: u32,
}

impl ResilienceConfig {
    pub fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::new(
            self.retry_max_attempts,
            self.retry_initial_backoff_ms,
            self.retry_backoff_multiplier,
        )
    }
}

/// Runs `operation` through retry-with-backoff while the circuit is closed
/// or half-open, then records the final outcome against `breaker`. When the
/// circuit is open, fails fast with `DomainError::ServiceUnavailable` without
/// invoking `operation` at all.
pub async fn call_with_resilience<F, Fut, T>(
    breaker: &CircuitBreaker,
    policy: &RetryPolicy,
    operation: F,
) -> Result<T, DomainError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, DomainError>>,
{
    if !breaker.allow_request() {
        return Err(DomainError::ServiceUnavailable);
    }

    let result = retry_with_backoff(
        policy,
        |err| matches!(err, DomainError::Repository(_)),
        operation,
    )
    .await;

    match &result {
        Ok(_) => breaker.record_success(),
        Err(DomainError::Repository(_)) => breaker.record_failure(),
        Err(_) => {}
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use circuit_breaker::CircuitState;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn policy() -> RetryPolicy {
        RetryPolicy::new(3, 1, 1)
    }

    #[tokio::test]
    async fn succeeds_and_keeps_circuit_closed() {
        let breaker = CircuitBreaker::new(3, 60_000);

        let result =
            call_with_resilience(&breaker, &policy(), || async { Ok::<_, DomainError>(1) }).await;

        assert_eq!(result, Ok(1));
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn opens_circuit_after_repeated_repository_failures() {
        let breaker = CircuitBreaker::new(1, 60_000);

        let result = call_with_resilience(&breaker, &policy(), || async {
            Err::<u32, _>(DomainError::Repository("boom".into()))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn fails_fast_without_calling_operation_when_circuit_is_open() {
        let breaker = CircuitBreaker::new(1, 60_000);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        let calls = AtomicU32::new(0);
        let result = call_with_resilience(&breaker, &policy(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok::<_, DomainError>(1) }
        })
        .await;

        assert_eq!(result, Err(DomainError::ServiceUnavailable));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn does_not_trip_the_breaker_on_non_repository_errors() {
        let breaker = CircuitBreaker::new(1, 60_000);

        let result = call_with_resilience(&breaker, &policy(), || async {
            Err::<u32, _>(DomainError::InvalidCredentials)
        })
        .await;

        assert_eq!(result, Err(DomainError::InvalidCredentials));
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}
