from __future__ import annotations
import base64
import hashlib
import hmac
from datetime import datetime, timezone
from urllib.parse import urlparse, urlencode, parse_qsl, quote

from ._models import Request

_OBS_SUB_RESOURCES = frozenset([
    "acl", "cors", "delete", "lifecycle", "location", "logging",
    "notification", "partNumber", "policy", "quota", "replication",
    "requestPayment", "restore", "storageClass", "storageinfo",
    "tagging", "torrent", "uploadId", "uploads", "versioning",
    "versionId", "website",
])


def build_canonicalized_resource(
    bucket: str, key: str, sub_resources: dict[str, str] | None = None
) -> str:
    path = f"/{bucket}/{key}" if key else f"/{bucket}/"
    if sub_resources:
        filtered = {k: v for k, v in sub_resources.items() if k in _OBS_SUB_RESOURCES}
        if filtered:
            parts = []
            for k in sorted(filtered):
                parts.append(k if filtered[k] == "" else f"{k}={filtered[k]}")
            path += "?" + "&".join(parts)
    return path


def build_string_to_sign(
    method: str,
    content_md5: str,
    content_type: str,
    date: str,
    obs_headers: dict[str, str],
    canonicalized_resource: str,
) -> str:
    canonicalized_obs_headers = ""
    if obs_headers:
        lines = sorted(
            f"{k.lower()}:{v.strip()}"
            for k, v in obs_headers.items()
            if k.lower().startswith("x-obs-")
        )
        if lines:
            canonicalized_obs_headers = "\n".join(lines) + "\n"
    return (
        f"{method}\n"
        f"{content_md5}\n"
        f"{content_type}\n"
        f"{date}\n"
        f"{canonicalized_obs_headers}"
        f"{canonicalized_resource}"
    )


def _sign(secret_key: str, string_to_sign: str) -> str:
    sig = hmac.new(
        secret_key.encode("utf-8"),
        string_to_sign.encode("utf-8"),
        hashlib.sha1,
    ).digest()
    return base64.b64encode(sig).decode("utf-8")


class SignerV2:
    def __init__(self, access_key: str, secret_key: str) -> None:
        self.access_key = access_key
        self.secret_key = secret_key

    def sign(self, request: Request) -> Request:
        parsed = urlparse(request.url)
        host = parsed.netloc
        # Detect bucket: virtual-hosted (bucket.obs.region.myhuaweicloud.com) or path
        if host.count(".") >= 3 and "obs." in host:
            bucket = host.split(".")[0]
            key = parsed.path.lstrip("/")
        else:
            path_parts = parsed.path.lstrip("/").split("/", 1)
            bucket = path_parts[0] if path_parts else ""
            key = path_parts[1] if len(path_parts) > 1 else ""

        headers = dict(request.headers)
        if "Date" not in headers:
            headers["Date"] = datetime.now(timezone.utc).strftime(
                "%a, %d %b %Y %H:%M:%S GMT"
            )

        all_params: dict[str, str] = {}
        if parsed.query:
            for k, v in parse_qsl(parsed.query, keep_blank_values=True):
                all_params[k] = v
        if request.params:
            for k, v in request.params.items():
                all_params[k] = str(v)

        obs_headers = {k: v for k, v in headers.items() if k.lower().startswith("x-obs-")}
        resource = build_canonicalized_resource(
            bucket=bucket, key=key, sub_resources=all_params or None
        )
        sts = build_string_to_sign(
            method=request.method,
            content_md5=headers.get("Content-MD5", ""),
            content_type=headers.get("Content-Type", ""),
            date=headers["Date"],
            obs_headers=obs_headers,
            canonicalized_resource=resource,
        )
        signature = _sign(self.secret_key, sts)
        headers["Authorization"] = f"OBS {self.access_key}:{signature}"

        wire_query = "&".join(
            f"{quote(k, safe='-_.~')}={quote(v, safe='-_.~')}" if v != "" else quote(k, safe="-_.~")
            for k, v in all_params.items()
        ) if all_params else ""
        signed_url = f"{parsed.scheme}://{parsed.netloc}{parsed.path or '/'}"
        if wire_query:
            signed_url += f"?{wire_query}"

        return Request(
            method=request.method,
            url=signed_url,
            headers=headers,
            params=None,
            body=request.body,
        )

    def presign(
        self,
        method: str,
        url: str,
        expires_unix: int,
        bucket: str,
        key: str,
        obs_headers: dict[str, str] | None = None,
    ) -> str:
        resource = build_canonicalized_resource(bucket=bucket, key=key)
        sts = build_string_to_sign(
            method=method,
            content_md5="",
            content_type="",
            date=str(expires_unix),
            obs_headers=obs_headers or {},
            canonicalized_resource=resource,
        )
        signature = _sign(self.secret_key, sts)
        parsed = urlparse(url)
        params = urlencode({
            "AccessKeyId": self.access_key,
            "Expires": str(expires_unix),
            "Signature": signature,
        })
        return f"{parsed.scheme}://{parsed.netloc}{parsed.path}?{params}"
