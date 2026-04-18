from __future__ import annotations
import re
from urllib.parse import quote


_OBS_DOMAIN_RE = re.compile(
    r"^https?://obs\.[a-z0-9-]+\.(myhuaweicloud\.com|myhuaweicloud\.eu)$"
)


def is_obs_domain(endpoint: str) -> bool:
    return bool(_OBS_DOMAIN_RE.match(endpoint.rstrip("/")))


def is_dns_safe_bucket(bucket: str) -> bool:
    return bool(re.match(r"^[a-z0-9][a-z0-9\-]{1,61}[a-z0-9]$", bucket))


def build_url(
    endpoint: str,
    bucket: str,
    key: str | None,
    addressing_style: str = "auto",
) -> str:
    endpoint = endpoint.rstrip("/")
    encoded_key = quote(key or "", safe="/") if key else ""

    use_virtual = (
        addressing_style == "virtual"
        or (
            addressing_style == "auto"
            and is_obs_domain(endpoint)
            and is_dns_safe_bucket(bucket)
        )
    )

    if use_virtual:
        scheme, rest = endpoint.split("://", 1)
        base = f"{scheme}://{bucket}.{rest}"
        return f"{base}/{encoded_key}"
    else:
        return f"{endpoint}/{bucket}/{encoded_key}"
