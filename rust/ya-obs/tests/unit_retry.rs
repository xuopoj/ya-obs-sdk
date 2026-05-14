use std::time::Duration;
use ya_obs::retry::{parse_retry_after, RetryDecision, RetryPolicy};

#[test]
fn retry_policy_retries_5xx() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(500, None, 0), RetryDecision::Retry(_)));
    assert!(matches!(p.classify(503, None, 0), RetryDecision::Retry(_)));
}

#[test]
fn retry_policy_retries_429_and_408() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(408, None, 0), RetryDecision::Retry(_)));
    assert!(matches!(p.classify(429, None, 0), RetryDecision::Retry(_)));
}

#[test]
fn retry_policy_does_not_retry_4xx_other() {
    let p = RetryPolicy::default();
    assert!(matches!(
        p.classify(404, None, 0),
        RetryDecision::DoNotRetry
    ));
    assert!(matches!(
        p.classify(403, None, 0),
        RetryDecision::DoNotRetry
    ));
}

#[test]
fn retry_policy_stops_after_max_attempts() {
    let p = RetryPolicy::default();
    assert!(matches!(
        p.classify(503, None, 2),
        RetryDecision::DoNotRetry
    ));
}

#[test]
fn retry_after_seconds_overrides_computed_delay() {
    let p = RetryPolicy::default();
    let d = p.classify(503, Some("7"), 0);
    match d {
        RetryDecision::Retry(delay) => assert_eq!(delay, Duration::from_secs(7)),
        _ => panic!("expected Retry"),
    }
}

#[test]
fn parse_retry_after_handles_seconds_and_http_date() {
    assert_eq!(parse_retry_after("5"), Some(Duration::from_secs(5)));
    let result = parse_retry_after("Wed, 21 Oct 2099 07:28:00 GMT");
    assert!(result.is_some());
}
