from __future__ import annotations
import os
import time
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import BinaryIO, Callable, Iterator

from ._errors import ObsError
from ._http import HttpClient, AsyncHttpClient
from ._models import Request, Timeout, RetryPolicy, RetryEvent
from ._responses import (
    PutObjectResponse, GetObjectResponse, HeadObjectResponse,
    DeleteObjectResponse, ListObjectsPage, ObjectInfo, CopyObjectResponse,
)
from ._signer_v4 import SignerV4
from ._signer_v2 import SignerV2
from ._streaming import StreamingBody
from ._url import build_url
from ._xml import parse_list_objects


def _resolve_credentials(access_key: str | None, secret_key: str | None) -> tuple[str, str]:
    ak = access_key or os.environ.get("HUAWEICLOUD_SDK_AK", "")
    sk = secret_key or os.environ.get("HUAWEICLOUD_SDK_SK", "")
    if not ak or not sk:
        raise ValueError(
            "OBS credentials not found. Pass access_key/secret_key or set "
            "HUAWEICLOUD_SDK_AK / HUAWEICLOUD_SDK_SK environment variables."
        )
    return ak, sk


def _resolve_endpoint(region: str | None, endpoint: str | None) -> str:
    if endpoint:
        return endpoint.rstrip("/")
    if region:
        return f"https://obs.{region}.myhuaweicloud.com"
    raise ValueError("Either region or endpoint must be provided.")


def _parse_last_modified(value: str) -> datetime:
    if not value:
        return datetime.now(timezone.utc)
    try:
        return parsedate_to_datetime(value)
    except Exception:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))


def _extract_metadata(headers: dict[str, str]) -> dict[str, str]:
    prefix = "x-obs-meta-"
    return {
        k[len(prefix):]: v
        for k, v in headers.items()
        if k.lower().startswith(prefix)
    }


def _make_signer(signing_version: str, ak: str, sk: str, region: str):
    if signing_version == "v2":
        return SignerV2(access_key=ak, secret_key=sk)
    return SignerV4(access_key=ak, secret_key=sk, region=region)


class Client:
    def __init__(
        self,
        access_key: str | None = None,
        secret_key: str | None = None,
        region: str | None = None,
        endpoint: str | None = None,
        addressing_style: str = "auto",
        signing_version: str = "v4",
        timeout: Timeout | None = None,
        retry_policy: RetryPolicy | None = None,
        on_retry: Callable[[RetryEvent], None] | None = None,
    ) -> None:
        ak, sk = _resolve_credentials(access_key, secret_key)
        self._endpoint = _resolve_endpoint(region, endpoint)
        self._addressing_style = addressing_style
        self._region = region or ""
        self._signer = _make_signer(signing_version, ak, sk, self._region)
        self._http = HttpClient(
            signer=self._signer,
            timeout=timeout or Timeout(),
            retry_policy=retry_policy or RetryPolicy(),
            on_retry=on_retry,
        )

    def _url(self, bucket: str, key: str | None = None) -> str:
        return build_url(
            endpoint=self._endpoint,
            bucket=bucket,
            key=key,
            addressing_style=self._addressing_style,
        )

    def put_object(
        self,
        bucket: str,
        key: str,
        body: bytes | str | BinaryIO | Path,
        *,
        content_type: str | None = None,
        content_encoding: str | None = None,
        cache_control: str | None = None,
        content_disposition: str | None = None,
        metadata: dict[str, str] | None = None,
        extra_headers: dict[str, str] | None = None,
        multipart_threshold: int | None = None,
        part_size: int | None = None,
        concurrency: int | None = None,
    ) -> PutObjectResponse:
        if isinstance(body, str):
            body_bytes = body.encode("utf-8")
        elif isinstance(body, Path):
            body_bytes = body.read_bytes()
        elif isinstance(body, bytes):
            body_bytes = body
        else:
            body_bytes = body.read()

        threshold = multipart_threshold or (100 * 1024 * 1024)
        if len(body_bytes) >= threshold:
            from ._multipart import multipart_upload
            return multipart_upload(
                client=self,
                bucket=bucket,
                key=key,
                body=body_bytes,
                content_type=content_type,
                metadata=metadata,
                extra_headers=extra_headers,
                part_size=part_size,
                concurrency=concurrency or 4,
            )

        headers: dict[str, str] = {}
        if content_type:
            headers["Content-Type"] = content_type
        if content_encoding:
            headers["Content-Encoding"] = content_encoding
        if cache_control:
            headers["Cache-Control"] = cache_control
        if content_disposition:
            headers["Content-Disposition"] = content_disposition
        if metadata:
            for k, v in metadata.items():
                headers[f"x-obs-meta-{k}"] = v
        if extra_headers:
            headers.update(extra_headers)

        req = Request(method="PUT", url=self._url(bucket, key), headers=headers, body=body_bytes)
        raw = self._http.send(req)
        return PutObjectResponse(
            etag=raw.headers.get("etag", ""),
            version_id=raw.headers.get("x-obs-version-id"),
            request_id=raw.headers.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def get_object(
        self,
        bucket: str,
        key: str,
        *,
        range: tuple[int, int | None] | None = None,
        if_match: str | None = None,
        if_none_match: str | None = None,
    ) -> GetObjectResponse:
        headers: dict[str, str] = {}
        if range is not None:
            start, end = range
            headers["Range"] = f"bytes={start}-{end}" if end is not None else f"bytes={start}-"
        if if_match:
            headers["If-Match"] = if_match
        if if_none_match:
            headers["If-None-Match"] = if_none_match

        req = Request(method="GET", url=self._url(bucket, key), headers=headers)
        raw = self._http.send(req)
        h = raw.headers
        return GetObjectResponse(
            body=StreamingBody(iterator=raw.iter_bytes()),
            content_type=h.get("content-type"),
            content_length=int(h["content-length"]) if "content-length" in h else None,
            etag=h.get("etag", ""),
            last_modified=_parse_last_modified(h.get("last-modified", "")),
            metadata=_extract_metadata(h),
            cache_control=h.get("cache-control"),
            content_encoding=h.get("content-encoding"),
            content_disposition=h.get("content-disposition"),
            request_id=h.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def head_object(self, bucket: str, key: str) -> HeadObjectResponse:
        req = Request(method="HEAD", url=self._url(bucket, key))
        raw = self._http.send(req)
        h = raw.headers
        return HeadObjectResponse(
            content_type=h.get("content-type"),
            content_length=int(h["content-length"]) if "content-length" in h else None,
            etag=h.get("etag", ""),
            last_modified=_parse_last_modified(h.get("last-modified", "")),
            metadata=_extract_metadata(h),
            cache_control=h.get("cache-control"),
            content_encoding=h.get("content-encoding"),
            content_disposition=h.get("content-disposition"),
            request_id=h.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def delete_object(self, bucket: str, key: str) -> DeleteObjectResponse:
        req = Request(method="DELETE", url=self._url(bucket, key))
        raw = self._http.send(req)
        return DeleteObjectResponse(
            request_id=raw.headers.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def list_objects_page(
        self,
        bucket: str,
        *,
        prefix: str | None = None,
        marker: str | None = None,
        max_keys: int = 1000,
    ) -> ListObjectsPage:
        params: dict[str, str] = {"max-keys": str(max_keys)}
        if prefix:
            params["prefix"] = prefix
        if marker:
            params["marker"] = marker
        req = Request(method="GET", url=self._url(bucket), params=params)
        raw = self._http.send(req)
        parsed = parse_list_objects(raw.read().decode("utf-8"))
        objects = [
            ObjectInfo(
                key=o["key"],
                etag=o["etag"],
                size=o["size"],
                last_modified=datetime.fromisoformat(o["last_modified"]),
            )
            for o in parsed["objects"]
        ]
        return ListObjectsPage(
            objects=objects,
            is_truncated=parsed["is_truncated"],
            next_marker=parsed["next_marker"],
            request_id=raw.headers.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def list_objects(
        self,
        bucket: str,
        *,
        prefix: str | None = None,
    ) -> Iterator[ObjectInfo]:
        marker: str | None = None
        while True:
            page = self.list_objects_page(bucket, prefix=prefix, marker=marker)
            yield from page.objects
            if not page.is_truncated or not page.next_marker:
                break
            marker = page.next_marker

    def copy_object(
        self,
        src_bucket: str,
        src_key: str,
        dst_bucket: str,
        dst_key: str,
        *,
        metadata_directive: str = "COPY",
        metadata: dict[str, str] | None = None,
    ) -> CopyObjectResponse:
        headers: dict[str, str] = {
            "x-obs-copy-source": f"/{src_bucket}/{src_key}",
            "x-obs-metadata-directive": metadata_directive,
        }
        if metadata and metadata_directive == "REPLACE":
            for k, v in metadata.items():
                headers[f"x-obs-meta-{k}"] = v
        req = Request(method="PUT", url=self._url(dst_bucket, dst_key), headers=headers)
        raw = self._http.send(req)
        h = raw.headers
        return CopyObjectResponse(
            etag=h.get("etag", ""),
            last_modified=_parse_last_modified(h.get("last-modified", "")),
            request_id=h.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    def presign_get_object(self, bucket: str, key: str, expires: int = 3600) -> str:
        url = self._url(bucket, key)
        if isinstance(self._signer, SignerV2):
            expires_unix = int(time.time()) + expires
            return self._signer.presign(
                method="GET", url=url, expires_unix=expires_unix,
                bucket=bucket, key=key,
            )
        return self._signer.presign(method="GET", url=url, expires=expires)

    def close(self) -> None:
        self._http.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


class AsyncClient:
    def __init__(
        self,
        access_key: str | None = None,
        secret_key: str | None = None,
        region: str | None = None,
        endpoint: str | None = None,
        addressing_style: str = "auto",
        signing_version: str = "v4",
        timeout: Timeout | None = None,
        retry_policy: RetryPolicy | None = None,
        on_retry=None,
    ) -> None:
        ak, sk = _resolve_credentials(access_key, secret_key)
        self._endpoint = _resolve_endpoint(region, endpoint)
        self._addressing_style = addressing_style
        self._region = region or ""
        self._signer = _make_signer(signing_version, ak, sk, self._region)
        self._http = AsyncHttpClient(
            signer=self._signer,
            timeout=timeout or Timeout(),
            retry_policy=retry_policy or RetryPolicy(),
            on_retry=on_retry,
        )

    def _url(self, bucket: str, key: str | None = None) -> str:
        return build_url(
            endpoint=self._endpoint,
            bucket=bucket,
            key=key,
            addressing_style=self._addressing_style,
        )

    async def put_object(
        self,
        bucket: str,
        key: str,
        body: bytes | str,
        *,
        content_type: str | None = None,
        content_encoding: str | None = None,
        cache_control: str | None = None,
        content_disposition: str | None = None,
        metadata: dict[str, str] | None = None,
        extra_headers: dict[str, str] | None = None,
    ) -> PutObjectResponse:
        if isinstance(body, str):
            body = body.encode("utf-8")
        headers: dict[str, str] = {}
        if content_type:
            headers["Content-Type"] = content_type
        if content_encoding:
            headers["Content-Encoding"] = content_encoding
        if cache_control:
            headers["Cache-Control"] = cache_control
        if content_disposition:
            headers["Content-Disposition"] = content_disposition
        if metadata:
            for k, v in metadata.items():
                headers[f"x-obs-meta-{k}"] = v
        if extra_headers:
            headers.update(extra_headers)
        req = Request(method="PUT", url=self._url(bucket, key), headers=headers, body=body)
        raw = await self._http.send(req)
        return PutObjectResponse(
            etag=raw.headers.get("etag", ""),
            version_id=raw.headers.get("x-obs-version-id"),
            request_id=raw.headers.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    async def get_object(
        self,
        bucket: str,
        key: str,
        *,
        range: tuple[int, int | None] | None = None,
    ) -> GetObjectResponse:
        headers: dict[str, str] = {}
        if range is not None:
            start, end = range
            headers["Range"] = f"bytes={start}-{end}" if end is not None else f"bytes={start}-"
        req = Request(method="GET", url=self._url(bucket, key), headers=headers)
        raw = await self._http.send(req)
        stream = raw.stream()
        h = raw.headers
        return GetObjectResponse(
            body=stream,
            content_type=h.get("content-type"),
            content_length=int(h["content-length"]) if "content-length" in h else None,
            etag=h.get("etag", ""),
            last_modified=_parse_last_modified(h.get("last-modified", "")),
            metadata=_extract_metadata(h),
            cache_control=h.get("cache-control"),
            content_encoding=h.get("content-encoding"),
            content_disposition=h.get("content-disposition"),
            request_id=h.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    async def delete_object(self, bucket: str, key: str) -> DeleteObjectResponse:
        req = Request(method="DELETE", url=self._url(bucket, key))
        raw = await self._http.send(req)
        return DeleteObjectResponse(
            request_id=raw.headers.get("x-obs-request-id", ""),
            client_request_id=raw.client_request_id,
        )

    async def close(self) -> None:
        await self._http.close()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        await self.close()
