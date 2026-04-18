from __future__ import annotations
import math
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .client import Client

from ._models import Request
from ._responses import PutObjectResponse
from ._xml import serialize_complete_multipart, parse_initiate_multipart

_MIN_PART_SIZE = 8 * 1024 * 1024
_MAX_PARTS = 9000


def compute_part_size(total_size: int) -> int:
    adaptive = math.ceil(total_size / _MAX_PARTS)
    return max(_MIN_PART_SIZE, adaptive)


def split_into_parts(data: bytes, part_size: int) -> list[tuple[int, bytes]]:
    parts = []
    for i, offset in enumerate(range(0, len(data), part_size), start=1):
        parts.append((i, data[offset:offset + part_size]))
    return parts


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
) -> PutObjectResponse:
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
    upload_id = parsed_init["upload_id"]

    actual_part_size = part_size or compute_part_size(len(body))
    parts_data = split_into_parts(body, actual_part_size)
    completed_parts: list[tuple[int, str]] = []

    def upload_part(part_number: int, chunk: bytes) -> tuple[int, str]:
        req = Request(
            method="PUT",
            url=client._url(bucket, key),
            params={"partNumber": str(part_number), "uploadId": upload_id},
            body=chunk,
        )
        r = client._http.send(req)
        return (part_number, r.headers.get("etag", ""))

    try:
        with ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = {
                pool.submit(upload_part, pn, chunk): pn
                for pn, chunk in parts_data
            }
            for future in as_completed(futures):
                pn, etag = future.result()
                completed_parts.append((pn, etag))
    except Exception:
        abort_req = Request(
            method="DELETE",
            url=client._url(bucket, key),
            params={"uploadId": upload_id},
        )
        try:
            client._http.send(abort_req)
        except Exception:
            pass
        raise

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
