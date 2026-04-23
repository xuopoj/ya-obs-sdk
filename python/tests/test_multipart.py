import math
import os
import tempfile
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from ya_obs._multipart import (
    compute_part_size,
    split_into_parts,
    multipart_upload_from_path,
)


pytestmark_posix = pytest.mark.skipif(
    not hasattr(os, "pread"), reason="pread not available on this platform"
)


def _make_mock_client(initiate_xml: bytes, part_etags: list[str], complete_etag: str):
    client = MagicMock()
    client._url = lambda bucket, key=None: f"https://test/{bucket}/{key or ''}"

    sent_requests = []
    etag_iter = iter(part_etags)

    def send(req):
        sent_requests.append(req)
        params = req.params or {}
        if req.method == "POST" and "uploads" in params:
            resp = MagicMock()
            resp.read = lambda: initiate_xml
            resp.headers = {}
            resp.client_request_id = "cid"
            return resp
        if req.method == "PUT" and "partNumber" in params:
            resp = MagicMock()
            resp.headers = {"etag": next(etag_iter)}
            resp.client_request_id = "cid"
            return resp
        if req.method == "POST" and "uploadId" in params:
            resp = MagicMock()
            resp.headers = {"etag": complete_etag, "x-obs-request-id": "req-final"}
            resp.client_request_id = "cid"
            return resp
        raise AssertionError(f"unexpected request: {req.method} {params}")

    client._http.send = send
    client._sent = sent_requests
    return client

_8MB = 8 * 1024 * 1024

def test_small_file_uses_8mb_parts():
    assert compute_part_size(total_size=50 * 1024 * 1024) == _8MB

def test_large_file_adapts_part_size():
    size = 100 * 1024 * 1024 * 1024
    part_size = compute_part_size(total_size=size)
    assert part_size > _8MB
    assert math.ceil(size / part_size) <= 9000

def test_split_into_parts_count():
    data = b"x" * (20 * 1024 * 1024)
    parts = split_into_parts(data, part_size=_8MB)
    assert len(parts) == 3
    assert len(parts[0][1]) == _8MB
    assert len(parts[2][1]) == 4 * 1024 * 1024

def test_split_into_parts_numbering():
    data = b"y" * (10 * 1024 * 1024)
    parts = split_into_parts(data, part_size=_8MB)
    assert parts[0][0] == 1
    assert parts[1][0] == 2

def test_split_into_parts_reconstructs():
    data = b"hello world " * 1000000
    parts = split_into_parts(data, part_size=_8MB)
    reconstructed = b"".join(chunk for _, chunk in parts)
    assert reconstructed == data


@pytestmark_posix
def test_multipart_upload_from_path_splits_correctly():
    part_size = 1024
    file_size = part_size * 3 + 100  # 4 parts: 1024, 1024, 1024, 100

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(b"A" * part_size)
        tmp.write(b"B" * part_size)
        tmp.write(b"C" * part_size)
        tmp.write(b"D" * 100)
        tmp_path = Path(tmp.name)

    try:
        initiate_xml = b'<?xml version="1.0"?><InitiateMultipartUploadResult><UploadId>up-1</UploadId></InitiateMultipartUploadResult>'
        client = _make_mock_client(
            initiate_xml=initiate_xml,
            part_etags=['"e1"', '"e2"', '"e3"', '"e4"'],
            complete_etag='"final-etag"',
        )

        result = multipart_upload_from_path(
            client=client,
            bucket="b",
            key="k",
            path=tmp_path,
            size=file_size,
            content_type="application/zip",
            metadata=None,
            extra_headers=None,
            part_size=part_size,
            concurrency=2,
        )

        assert result.etag == '"final-etag"'
        part_uploads = [r for r in client._sent if r.method == "PUT" and "partNumber" in (r.params or {})]
        assert len(part_uploads) == 4
        bodies_by_part = {int(r.params["partNumber"]): r.body for r in part_uploads}
        assert bodies_by_part[1] == b"A" * part_size
        assert bodies_by_part[2] == b"B" * part_size
        assert bodies_by_part[3] == b"C" * part_size
        assert bodies_by_part[4] == b"D" * 100
    finally:
        tmp_path.unlink()
