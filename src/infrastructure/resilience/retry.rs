use std::future::Future;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub backoff_multiplier: u32,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, initial_backoff_ms: u64, backoff_multiplier: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            initial_backoff_ms,
            backoff_multiplier: backoff_multiplier.max(1),
        }
    }

    fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let factor = self.backoff_multiplier.saturating_pow(attempt);
        Duration::from_millis(self.initial_backoff_ms.saturating_mul(factor as u64))
    }
}

/// Retries `operation` up to `policy.max_attempts` times with exponential
/// backoff between attempts, retrying only errors for which `is_retryable`
/// returns `true`. The first non-retryable error or the final attempt's
/// error is returned to the caller.
pub async fn retry_with_backoff<F, Fut, T, E>(
    policy: &RetryPolicy,
    is_retryable: impl Fn(&E) -> bool,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) if attempt + 1 < policy.max_attempts && is_retryable(&err) => {
                tokio::time::sleep(policy.backoff_for_attempt(attempt)).await;
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn returns_immediately_on_success() {
        let policy = RetryPolicy::new(3, 1, 1);
        let calls = AtomicU32::new(0);

        let result: Result<u32, &str> = retry_with_backoff(
            &policy,
            |_| true,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Ok(42) }
            },
        )
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_retryable_errors_until_max_attempts() {
        let policy = RetryPolicy::new(3, 1, 1);
        let calls = AtomicU32::new(0);

        let result: Result<u32, &str> = retry_with_backoff(
            &policy,
            |_| true,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err("transient") }
            },
        )
        .await;

        assert_eq!(result, Err("transient"));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stops_retrying_once_an_attempt_succeeds() {
        let policy = RetryPolicy::new(5, 1, 1);
        let calls = AtomicU32::new(0);

        let result: Result<u32, &str> = retry_with_backoff(
            &policy,
            |_| true,
            || {
                let attempt = calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    if attempt < 2 {
                        Err("transient")
                    } else {
                        Ok(7)
                    }
                }
            },
        )
        .await;

        assert_eq!(result, Ok(7));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable_errors() {
        let policy = RetryPolicy::new(5, 1, 1);
        let calls = AtomicU32::new(0);

        let result: Result<u32, &str> = retry_with_backoff(
            &policy,
            |_| false,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err("permanent") }
            },
        )
        .await;

        assert_eq!(result, Err("permanent"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
