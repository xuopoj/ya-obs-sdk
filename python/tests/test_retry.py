import pytest
from unittest.mock import patch, AsyncMock
from ya_obs._retry import RetryLoop, AsyncRetryLoop, is_retryable_status, compute_delay, parse_retry_after
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


def test_retry_loop_honors_retry_after_on_error():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=2, base_delay=10.0, jitter=False)
    loop = RetryLoop(policy=policy)
    err = ServerError(code="SlowDown", message="x", status=503)
    err._retry_after = 0.25
    calls = []
    def operation():
        calls.append(1)
        if len(calls) == 1:
            raise err
        return "ok"
    with patch("ya_obs._retry.time.sleep") as sleep:
        loop.run(operation)
    sleep.assert_called_once_with(0.25)


def test_parse_retry_after_seconds():
    assert parse_retry_after("5") == 5.0
    assert parse_retry_after("0.5") == 0.5
    assert parse_retry_after("  3  ") == 3.0


def test_parse_retry_after_negative_clamped_to_zero():
    assert parse_retry_after("-1") == 0.0


def test_parse_retry_after_invalid_or_empty():
    assert parse_retry_after(None) is None
    assert parse_retry_after("") is None
    assert parse_retry_after("not-a-date") is None


def test_parse_retry_after_http_date_in_past_returns_zero():
    assert parse_retry_after("Wed, 01 Jan 2020 00:00:00 GMT") == 0.0


async def test_async_retry_loop_succeeds_first_try():
    loop = AsyncRetryLoop(policy=RetryPolicy(max_attempts=3))
    calls = []
    async def op():
        calls.append(1)
        return "ok"
    assert await loop.run(op) == "ok"
    assert calls == [1]


async def test_async_retry_loop_retries_on_retryable_error():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=3, base_delay=0.01, jitter=False)
    loop = AsyncRetryLoop(policy=policy)
    calls = []
    async def op():
        calls.append(1)
        if len(calls) < 3:
            raise ServerError(code="InternalError", message="oops", status=500)
        return "ok"
    with patch("ya_obs._retry.asyncio.sleep", new=AsyncMock()):
        assert await loop.run(op) == "ok"
    assert len(calls) == 3


async def test_async_retry_loop_raises_after_max_attempts():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=2, base_delay=0.01, jitter=False)
    loop = AsyncRetryLoop(policy=policy)
    async def op():
        raise ServerError(code="InternalError", message="oops", status=500)
    with patch("ya_obs._retry.asyncio.sleep", new=AsyncMock()):
        with pytest.raises(ServerError):
            await loop.run(op)


async def test_async_retry_loop_does_not_retry_client_errors():
    from ya_obs._errors import NoSuchKey
    loop = AsyncRetryLoop(policy=RetryPolicy(max_attempts=3))
    calls = []
    async def op():
        calls.append(1)
        raise NoSuchKey(code="NoSuchKey", message="x", status=404)
    with pytest.raises(NoSuchKey):
        await loop.run(op)
    assert calls == [1]
