from __future__ import annotations
import logging
import random
import time
from typing import Callable, TypeVar

from ._errors import ObsError, ClientError
from ._models import RetryPolicy, RetryEvent

logger = logging.getLogger("ya_obs")
T = TypeVar("T")

_RETRYABLE_STATUSES = frozenset([408, 429, 500, 502, 503, 504])


def is_retryable_status(status: int) -> bool:
    return status in _RETRYABLE_STATUSES


def compute_delay(attempt: int, policy: RetryPolicy, retry_after: float | None) -> float:
    if retry_after is not None:
        return retry_after
    base = policy.base_delay * (2 ** (attempt - 1))
    capped = min(base, policy.max_delay)
    if policy.jitter:
        return random.uniform(0, capped)
    return capped


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
            except ClientError:
                raise
            except ObsError as e:
                if not is_retryable_status(e.status):
                    raise
                last_error = e
            except (ConnectionError, TimeoutError, OSError) as e:
                last_error = e

            if attempt == self.policy.max_attempts:
                break

            retry_after = getattr(last_error, "_retry_after", None)
            delay = compute_delay(attempt=attempt, policy=self.policy, retry_after=retry_after)
            event = RetryEvent(
                attempt=attempt,
                error=last_error,
                delay=delay,
                request_id=getattr(last_error, "request_id", None),
            )
            logger.warning("ya_obs: retry attempt %d after %.2fs — %s", attempt, delay, last_error)
            if self.on_retry:
                self.on_retry(event)
            time.sleep(delay)

        raise last_error
