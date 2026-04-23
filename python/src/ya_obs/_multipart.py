from __future__ import annotations
import math
import os
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import TYPE_CHECKING, Callable

if TYPE_CHECKING:
    from .client import Client

from ._models import ProgressEvent, Request
from ._responses import PutObjectResponse
from ._xml import serialize_complete_multipart, parse_initiate_multipart

_MIN_PART_SIZE = 8 * 1024 * 1024
_MAX_PARTS = 9000


class _ProgressTracker:
    def __init__(
        self,
        total_bytes: int,
        callback: Callable[[ProgressEvent], None] | None,
    ) -> None:
        self.total = total_bytes
        self.callback = callback
        self.lock = threading.Lock()
        self.done = 0

    def report(self, chunk_size: int, part_number: int | None) -> None:
        if self.callback is None:
            return
        with self.lock:
            self.done += chunk_size
            transferred = self.done
        self.callback(ProgressEvent(
            bytes_transferred=transferred,
            total_bytes=self.total,
            part_number=part_number,
        ))


def compute_part_size(total_size: int) -> int:
    adaptive = math.ceil(total_size / _MAX_PARTS)
    return max(_MIN_PART_SIZE, adaptive)


def split_into_parts(data: bytes, part_size: int) -> list[tuple[int, bytes]]:
    parts = []
    for i, offset in enumerate(range(0, len(data), part_size), start=1):
        parts.append((i, data[offset:offset + part_size]))
    return parts


def _initiate(
    client: "Client",
    bucket: str,
    key: str,
    content_type: str | None,
    metadata: dict[str, str] | None,
    extra_headers: dict[str, str] | None,
) -> str:
    init_headers: dict[str, str] = {}
    if content_type:
        init_headers["Content-Type"] = content_type
    if metadata:
        for k, v in metadata.items():
            init_headers[f"x-obs-meta-{k}"] = v
    if extra_headers:
        init_headers.update(extra_headers)

    init_req = Request(
        method="POST",
        url=client._url(bucket, key),
        headers=init_headers,
        params={"uploads": ""},
    )
    raw = client._http.send(init_req)
    parsed_init = parse_initiate_multipart(raw.read().decode("utf-8"))
    return parsed_init["upload_id"]


def _abort(client: "Client", bucket: str, key: str, upload_id: str) -> None:
    abort_req = Request(
        method="DELETE",
        url=client._url(bucket, key),
        params={"uploadId": upload_id},
    )
    try:
        client._http.send(abort_req)
    except Exception:
        pass


def _complete(
    client: "Client",
    bucket: str,
    key: str,
    upload_id: str,
    completed_parts: list[tuple[int, str]],
) -> PutObjectResponse:
    completed_parts.sort(key=lambda x: x[0])
    xml_body = serialize_complete_multipart(completed_parts)
    complete_req = Request(
        method="POST",
        url=client._url(bucket, key),
        params={"uploadId": upload_id},
        headers={"Content-Type": "application/xml"},
        body=xml_body.encode("utf-8"),
    )
    raw_complete = client._http.send(complete_req)
    h = raw_complete.headers
    return PutObjectResponse(
        etag=h.get("etag", ""),
        version_id=h.get("x-obs-version-id"),
        request_id=h.get("x-obs-request-id", ""),
        client_request_id=raw_complete.client_request_id,
    )


def multipart_upload(
    client: "Client",
    bucket: str,
    key: str,
    body: bytes,
    content_type: str | None,
    metadata: dict[str, str] | None,
    extra_headers: dict[str, str] | None,
    part_size: int | None,
    concurrency: int,
    on_progress: Callable[[ProgressEvent], None] | None = None,
) -> PutObjectResponse:
    upload_id = _initiate(client, bucket, key, content_type, metadata, extra_headers)

    total_size = len(body)
    actual_part_size = part_size or compute_part_size(total_size)
    parts_data = split_into_parts(body, actual_part_size)
    completed_parts: list[tuple[int, str]] = []

    progress = _ProgressTracker(total_size, on_progress)

    def upload_part(part_number: int, chunk: bytes) -> tuple[int, str, int]:
        req = Request(
            method="PUT",
            url=client._url(bucket, key),
            params={"partNumber": str(part_number), "uploadId": upload_id},
            body=chunk,
        )
        r = client._http.send(req)
        return (part_number, r.headers.get("etag", ""), len(chunk))

    try:
        with ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = {
                pool.submit(upload_part, pn, chunk): pn
                for pn, chunk in parts_data
            }
            for future in as_completed(futures):
                pn, etag, chunk_size = future.result()
                completed_parts.append((pn, etag))
                progress.report(chunk_size, pn)
    except Exception:
        _abort(client, bucket, key, upload_id)
        raise

    return _complete(client, bucket, key, upload_id, completed_parts)


def multipart_upload_from_path(
    client: "Client",
    bucket: str,
    key: str,
    path: Path,
    size: int,
    content_type: str | None,
    metadata: dict[str, str] | None,
    extra_headers: dict[str, str] | None,
    part_size: int | None,
    concurrency: int,
    on_progress: Callable[[ProgressEvent], None] | None = None,
) -> PutObjectResponse:
    if not hasattr(os, "pread"):
        raise NotImplementedError(
            "path-based multipart upload requires os.pread (POSIX only); "
            "pass `bytes` or upload from a POSIX host"
        )

    actual_part_size = part_size or compute_part_size(size)
    part_count = math.ceil(size / actual_part_size)

    upload_id = _initiate(client, bucket, key, content_type, metadata, extra_headers)
    progress = _ProgressTracker(size, on_progress)

    def upload_part(part_number: int) -> tuple[int, str, int]:
        offset = (part_number - 1) * actual_part_size
        length = min(actual_part_size, size - offset)
        with open(path, "rb") as f:
            chunk = os.pread(f.fileno(), length, offset)
        req = Request(
            method="PUT",
            url=client._url(bucket, key),
            params={"partNumber": str(part_number), "uploadId": upload_id},
            body=chunk,
        )
        r = client._http.send(req)
        return (part_number, r.headers.get("etag", ""), length)

    completed_parts: list[tuple[int, str]] = []
    try:
        with ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = [pool.submit(upload_part, pn) for pn in range(1, part_count + 1)]
            for future in as_completed(futures):
                pn, etag, chunk_size = future.result()
                completed_parts.append((pn, etag))
                progress.report(chunk_size, pn)
    except Exception:
        _abort(client, bucket, key, upload_id)
        raise

    return _complete(client, bucket, key, upload_id, completed_parts)
