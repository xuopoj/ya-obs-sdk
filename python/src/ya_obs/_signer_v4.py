from __future__ import annotations
import hashlib
import hmac
from datetime import datetime, timezone
from urllib.parse import urlparse, urlencode

from ._models import Request


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _hmac_sha256(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode("utf-8"), hashlib.sha256).digest()


def signing_key(secret_key: str, date_str: str, region: str, service: str) -> bytes:
    k_date = _hmac_sha256(f"AWS4{secret_key}".encode("utf-8"), date_str)
    k_region = _hmac_sha256(k_date, region)
    k_service = _hmac_sha256(k_region, service)
    return _hmac_sha256(k_service, "aws4_request")


def canonical_request(
    method: str,
    path: str,
    query_string: str,
    headers: dict[str, str],
    signed_headers: str,
    body_sha256: str,
) -> str:
    signed_header_set = set(signed_headers.split(";"))
    sorted_headers = "\n".join(
        f"{k.lower()}:{v.strip()}"
        for k, v in sorted(headers.items(), key=lambda x: x[0].lower())
        if k.lower() in signed_header_set
    )
    return "\n".join([
        method,
        path,
        query_string,
        sorted_headers + "\n",
        signed_headers,
        body_sha256,
    ])


def string_to_sign(
    datetime_str: str,
    date_str: str,
    region: str,
    service: str,
    canonical_request: str,
) -> str:
    scope = f"{date_str}/{region}/{service}/aws4_request"
    cr_hash = _sha256(canonical_request.encode("utf-8"))
    return f"AWS4-HMAC-SHA256\n{datetime_str}\n{scope}\n{cr_hash}"


class SignerV4:
    def __init__(self, access_key: str, secret_key: str, region: str, service: str = "s3") -> None:
        self.access_key = access_key
        self.secret_key = secret_key
        self.region = region
        self.service = service

    def sign(self, request: Request) -> Request:
        now = datetime.now(timezone.utc)
        datetime_str = now.strftime("%Y%m%dT%H%M%SZ")
        date_str = now.strftime("%Y%m%d")

        parsed = urlparse(request.url)
        path = parsed.path or "/"

        body_sha256 = _sha256(request.body or b"")
        headers = dict(request.headers)
        headers["Host"] = parsed.netloc
        headers["x-amz-date"] = datetime_str
        headers["x-amz-content-sha256"] = body_sha256

        signed_header_names = ";".join(sorted(k.lower() for k in headers))

        cr = canonical_request(
            method=request.method,
            path=path,
            query_string=parsed.query or "",
            headers=headers,
            signed_headers=signed_header_names,
            body_sha256=body_sha256,
        )

        sts = string_to_sign(
            datetime_str=datetime_str,
            date_str=date_str,
            region=self.region,
            service=self.service,
            canonical_request=cr,
        )

        sk = signing_key(self.secret_key, date_str, self.region, self.service)
        signature = hmac.new(sk, sts.encode("utf-8"), hashlib.sha256).hexdigest()

        scope = f"{date_str}/{self.region}/{self.service}/aws4_request"
        auth = (
            f"AWS4-HMAC-SHA256 Credential={self.access_key}/{scope}, "
            f"SignedHeaders={signed_header_names}, Signature={signature}"
        )
        headers["Authorization"] = auth

        return Request(
            method=request.method,
            url=request.url,
            headers=headers,
            params=request.params,
            body=request.body,
        )

    def presign(self, method: str, url: str, expires: int) -> str:
        now = datetime.now(timezone.utc)
        datetime_str = now.strftime("%Y%m%dT%H%M%SZ")
        date_str = now.strftime("%Y%m%d")

        parsed = urlparse(url)
        path = parsed.path or "/"
        scope = f"{date_str}/{self.region}/{self.service}/aws4_request"
        signed_headers = "host"
        host = parsed.netloc

        params = {
            "X-Amz-Algorithm": "AWS4-HMAC-SHA256",
            "X-Amz-Credential": f"{self.access_key}/{scope}",
            "X-Amz-Date": datetime_str,
            "X-Amz-Expires": str(expires),
            "X-Amz-SignedHeaders": signed_headers,
        }
        query_string = urlencode(sorted(params.items()))

        cr = canonical_request(
            method=method,
            path=path,
            query_string=query_string,
            headers={"host": host},
            signed_headers=signed_headers,
            body_sha256="UNSIGNED-PAYLOAD",
        )

        sts = string_to_sign(
            datetime_str=datetime_str,
            date_str=date_str,
            region=self.region,
            service=self.service,
            canonical_request=cr,
        )

        sk = signing_key(self.secret_key, date_str, self.region, self.service)
        signature = hmac.new(sk, sts.encode("utf-8"), hashlib.sha256).hexdigest()
        return f"{parsed.scheme}://{host}{path}?{query_string}&X-Amz-Signature={signature}"
