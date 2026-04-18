from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Request:
    method: str
    url: str
    headers: dict[str, str] = field(default_factory=dict)
    params: dict[str, str] = field(default_factory=dict)
    body: bytes | None = None


@dataclass
class Timeout:
    connect: float = 10.0
    read: float = 60.0
    total: float | None = None


@dataclass
class RetryPolicy:
    max_attempts: int = 3
    base_delay: float = 0.5
    max_delay: float = 30.0
    jitter: bool = True


@dataclass
class RetryEvent:
    attempt: int
    error: Exception
    delay: float
    request_id: str | None
