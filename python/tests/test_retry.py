import pytest
from unittest.mock import patch
from ya_obs._retry import RetryLoop, is_retryable_status, compute_delay
from ya_obs._models import RetryPolicy

def test_is_retryable_status():
    assert is_retryable_status(500) is True
    assert is_retryable_status(503) is True
    assert is_retryable_status(408) is True
    assert is_retryable_status(429) is True
    assert is_retryable_status(404) is False
    assert is_retryable_status(403) is False
    assert is_retryable_status(200) is False

def test_compute_delay_without_jitter():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=False)
    assert compute_delay(attempt=1, policy=policy, retry_after=None) == 1.0
    assert compute_delay(attempt=2, policy=policy, retry_after=None) == 2.0
    assert compute_delay(attempt=3, policy=policy, retry_after=None) == 4.0

def test_compute_delay_caps_at_max():
    policy = RetryPolicy(base_delay=10.0, max_delay=15.0, jitter=False)
    assert compute_delay(attempt=3, policy=policy, retry_after=None) == 15.0

def test_compute_delay_respects_retry_after():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=False)
    assert compute_delay(attempt=1, policy=policy, retry_after=5.0) == 5.0

def test_compute_delay_with_jitter():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=True)
    delays = {compute_delay(attempt=1, policy=policy, retry_after=None) for _ in range(20)}
    assert len(delays) > 1
    assert all(0 <= d <= 1.0 for d in delays)

def test_retry_loop_succeeds_on_first_try():
    policy = RetryPolicy(max_attempts=3)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        return "ok"
    result = loop.run(operation)
    assert result == "ok"
    assert len(calls) == 1

def test_retry_loop_retries_on_retryable_error():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=3, base_delay=0.01, jitter=False)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        if len(calls) < 3:
            raise ServerError(code="InternalError", message="oops", status=500)
        return "ok"
    with patch("ya_obs._retry.time.sleep"):
        result = loop.run(operation)
    assert result == "ok"
    assert len(calls) == 3

def test_retry_loop_raises_after_max_attempts():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=2, base_delay=0.01, jitter=False)
    loop = RetryLoop(policy=policy)
    def operation():
        raise ServerError(code="InternalError", message="oops", status=500)
    with patch("ya_obs._retry.time.sleep"):
        with pytest.raises(ServerError):
            loop.run(operation)

def test_retry_loop_does_not_retry_client_errors():
    from ya_obs._errors import NoSuchKey
    policy = RetryPolicy(max_attempts=3)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        raise NoSuchKey(code="NoSuchKey", message="not found", status=404)
    with pytest.raises(NoSuchKey):
        loop.run(operation)
    assert len(calls) == 1
