from __future__ import annotations
import asyncio
import email.utils
import logging
import random
import time
from typing import Awaitable, Callable, TypeVar

from ._errors import ObsError, ClientError
from ._models import RetryPolicy, RetryEvent

logger = logging.getLogger("ya_obs")
T = TypeVar("T")

_RETRYABLE_STATUSES = frozenset([408, 429, 500, 502, 503, 504])


def is_retryable_status(status: int) -> bool:
    return status in _RETRYABLE_STATUSES


def parse_retry_after(value: str | None) -> float | None:
    if not value:
        return None
    value = value.strip()
    try:
        return max(0.0, float(value))
    except ValueError:
        pass
    try:
        parsed = email.utils.parsedate_to_datetime(value)
    except (TypeError, ValueError):
        return None
    if parsed is None:
        return None
    delay = parsed.timestamp() - time.time()
    return max(0.0, delay)


def compute_delay(attempt: int, policy: RetryPolicy, retry_after: float | None) -> float:
    if retry_after is not None:
        return retry_after
    base = policy.base_delay * (2 ** (attempt - 1))
    capped = min(base, policy.max_delay)
    if policy.jitter:
        return random.uniform(0, capped)
    return capped


def _classify(error: Exception) -> Exception | None:
    if isinstance(error, ClientError):
        raise error
    if isinstance(error, ObsError):
        if not is_retryable_status(error.status):
            raise error
        return error
    if isinstance(error, (ConnectionError, TimeoutError, OSError)):
        return error
    raise error


def _next_event(attempt: int, policy: RetryPolicy, last_error: Exception) -> RetryEvent:
    retry_after = getattr(last_error, "_retry_after", None)
    delay = compute_delay(attempt=attempt, policy=policy, retry_after=retry_after)
    return RetryEvent(
        attempt=attempt,
        error=last_error,
        delay=delay,
        request_id=getattr(last_error, "request_id", None),
    )


class RetryLoop:
    def __init__(
        self,
        policy: RetryPolicy,
        on_retry: Callable[[RetryEvent], None] | None = None,
    ) -> None:
        self.policy = policy
        self.on_retry = on_retry

    def run(self, operation: Callable[[], T]) -> T:
        last_error: Exception | None = None
        for attempt in range(1, self.policy.max_attempts + 1):
            try:
                return operation()
            except Exception as e:
                last_error = _classify(e)

            if attempt == self.policy.max_attempts:
                break

            event = _next_event(attempt, self.policy, last_error)
            logger.warning("ya_obs: retry attempt %d after %.2fs — %s", attempt, event.delay, last_error)
            if self.on_retry:
                self.on_retry(event)
            time.sleep(event.delay)

        raise last_error


class AsyncRetryLoop:
    def __init__(
        self,
        policy: RetryPolicy,
        on_retry: Callable[[RetryEvent], None] | None = None,
    ) -> None:
        self.policy = policy
        self.on_retry = on_retry

    async def run(self, operation: Callable[[], Awaitable[T]]) -> T:
        last_error: Exception | None = None
        for attempt in range(1, self.policy.max_attempts + 1):
            try:
                return await operation()
            except Exception as e:
                last_error = _classify(e)

            if attempt == self.policy.max_attempts:
                break

            event = _next_event(attempt, self.policy, last_error)
            logger.warning("ya_obs: retry attempt %d after %.2fs — %s", attempt, event.delay, last_error)
            if self.on_retry:
                self.on_retry(event)
            await asyncio.sleep(event.delay)

        raise last_error
