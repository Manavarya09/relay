//! Retry with exponential backoff for API requests.

use std::thread;
use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
        }
    }
}

/// HTTP status codes that are safe to retry.
fn is_retryable_status(code: u16) -> bool {
    matches!(code, 429 | 500 | 502 | 503 | 504)
}

/// Classify a ureq error as retryable or not.
pub fn is_retryable_error(err: &ureq::Error) -> bool {
    match err {
        ureq::Error::Status(code, _) => is_retryable_status(*code),
        ureq::Error::Transport(_) => true, // Network errors are retryable
    }
}

/// Execute an HTTP request with retry and exponential backoff.
/// Returns the response on success, or the last error after all retries exhausted.
pub fn with_retry<F>(config: &RetryConfig, mut request_fn: F) -> Result<ureq::Response, ureq::Error>
where
    F: FnMut() -> Result<ureq::Response, ureq::Error>,
{
    let mut last_err = None;

    for attempt in 0..=config.max_retries {
        match request_fn() {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                if attempt < config.max_retries && is_retryable_error(&e) {
                    let delay = config.base_delay_ms * 2u64.pow(attempt);
                    tracing::warn!(
                        "Request failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1, config.max_retries + 1, delay, e
                    );
                    thread::sleep(Duration::from_millis(delay));
                    last_err = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 1000);
    }

    #[test]
    fn retryable_status_codes() {
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(404));
    }
}
