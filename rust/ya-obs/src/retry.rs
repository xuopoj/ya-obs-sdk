use std::time::{Duration, SystemTime};

use time::{format_description::well_known::Rfc2822, OffsetDateTime};

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RetryDecision {
    Retry(Duration),
    DoNotRetry,
}

impl RetryPolicy {
    /// Decide what to do based on status, optional Retry-After header, and the
    /// number of attempts already made (0 = first call just failed).
    pub fn classify(
        &self,
        status: u16,
        retry_after: Option<&str>,
        attempts_made: u32,
    ) -> RetryDecision {
        if attempts_made + 1 >= self.max_attempts {
            return RetryDecision::DoNotRetry;
        }

        let retryable = status >= 500 || status == 408 || status == 429;
        if !retryable {
            return RetryDecision::DoNotRetry;
        }

        if let Some(h) = retry_after.and_then(parse_retry_after) {
            return RetryDecision::Retry(h.min(self.max_delay));
        }

        let exp = self.base_delay.saturating_mul(1 << attempts_made);
        let capped = exp.min(self.max_delay);
        let jittered_ms = fastrand::u64(..=capped.as_millis() as u64);
        RetryDecision::Retry(Duration::from_millis(jittered_ms))
    }
}

/// Parse the `Retry-After` header. Returns seconds or an HTTP-date converted
/// to a duration from now (saturating at 0).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(secs) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let dt = OffsetDateTime::parse(value.trim(), &Rfc2822).ok()?;
    let target: SystemTime = dt.into();
    target
        .duration_since(SystemTime::now())
        .ok()
        .or(Some(Duration::ZERO))
}
