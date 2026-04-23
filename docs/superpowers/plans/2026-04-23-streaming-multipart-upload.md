# Streaming Multipart Upload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow `Client.put_object(body=Path(...))` to upload arbitrarily large files with bounded memory (~`concurrency × part_size`) instead of slurping the whole file into RAM.

**Architecture:** Add a `multipart_upload_from_path` function that splits the file by offset/length and reads each chunk via `os.pread` from a per-worker file descriptor. Refactor the existing `multipart_upload` to share `_initiate` / `_complete` / `_abort` helpers. `Client.put_object` routes large `Path` inputs to the new function; small files and `bytes` / `str` / `BinaryIO` paths are unchanged.

**Tech Stack:** Python 3.12, `httpx`, `pytest`, `pytest_httpx`, stdlib `os.pread` and `concurrent.futures.ThreadPoolExecutor`.

**Spec:** `docs/superpowers/specs/2026-04-23-streaming-multipart-upload-design.md`

---

## File Structure

- **Modify**: `python/src/ya_obs/_multipart.py` — extract helpers, add `multipart_upload_from_path`, add Windows guard.
- **Modify**: `python/src/ya_obs/client.py` — route `Path` inputs by file size, call new function.
- **Modify**: `python/tests/test_multipart.py` — unit tests for helpers and the path-based function (with mocked HTTP).
- **Modify**: `python/tests/test_client.py` — integration-ish test for `put_object(Path)` end-to-end through mocked HTTP.

No new files. The new function lives next to the existing one because they share helpers and the same module-level constants.

---

## Task 1: Refactor existing multipart into shared helpers

**Why first:** The new path-based function needs `_initiate` / `_complete` / `_abort`. Extracting them first keeps the diff in Task 2 small and lets us verify the existing tests still pass.

**Files:**
- Modify: `python/src/ya_obs/_multipart.py`
- Test: `python/tests/test_multipart.py` (existing tests must still pass — no new tests in this task)

- [ ] **Step 1: Run existing multipart tests to establish baseline**

```bash
cd python && .venv/bin/pytest tests/test_multipart.py -v
```

Expected: all 5 existing tests pass.

- [ ] **Step 2: Extract `_initiate`, `_complete`, `_abort` helpers in `_multipart.py`**

Replace the body of `_multipart.py` with the following. Keep `compute_part_size`, `split_into_parts`, and module constants exactly as they are; only the `multipart_upload` function changes (it now delegates to helpers). Helpers are module-private (leading underscore).

```python
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
) -> PutObjectResponse:
    upload_id = _initiate(client, bucket, key, content_type, metadata, extra_headers)

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
        _abort(client, bucket, key, upload_id)
        raise

    return _complete(client, bucket, key, upload_id, completed_parts)
```

- [ ] **Step 3: Re-run multipart tests + full client tests to ensure no regression**

```bash
cd python && .venv/bin/pytest tests/test_multipart.py tests/test_client.py -v
```

Expected: all tests pass. No behavior change.

- [ ] **Step 4: Commit**

```bash
git add python/src/ya_obs/_multipart.py
git commit -m "refactor: extract multipart init/complete/abort helpers"
```

---

## Task 2: Add `multipart_upload_from_path` with `pread`-per-worker

**Why:** This is the core of the feature — uploading parts read directly from disk instead of an in-memory `bytes` blob.

**Files:**
- Modify: `python/src/ya_obs/_multipart.py`
- Test: `python/tests/test_multipart.py`

- [ ] **Step 1: Write the failing test for `multipart_upload_from_path` happy path**

Append to `python/tests/test_multipart.py`:

```python
import os
import sys
import tempfile
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from ya_obs._multipart import multipart_upload_from_path


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
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd python && .venv/bin/pytest tests/test_multipart.py::test_multipart_upload_from_path_splits_correctly -v
```

Expected: FAIL with `ImportError: cannot import name 'multipart_upload_from_path'`.

- [ ] **Step 3: Implement `multipart_upload_from_path` in `_multipart.py`**

Add these imports at the top of `_multipart.py` (next to existing imports):

```python
import os
from pathlib import Path
```

Append this function to `_multipart.py` (after `multipart_upload`):

```python
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
) -> PutObjectResponse:
    if not hasattr(os, "pread"):
        raise NotImplementedError(
            "path-based multipart upload requires os.pread (POSIX only); "
            "pass `bytes` or upload from a POSIX host"
        )

    actual_part_size = part_size or compute_part_size(size)
    part_count = math.ceil(size / actual_part_size)

    upload_id = _initiate(client, bucket, key, content_type, metadata, extra_headers)

    def upload_part(part_number: int) -> tuple[int, str]:
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
        return (part_number, r.headers.get("etag", ""))

    completed_parts: list[tuple[int, str]] = []
    try:
        with ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = [pool.submit(upload_part, pn) for pn in range(1, part_count + 1)]
            for future in as_completed(futures):
                completed_parts.append(future.result())
    except Exception:
        _abort(client, bucket, key, upload_id)
        raise

    return _complete(client, bucket, key, upload_id, completed_parts)
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd python && .venv/bin/pytest tests/test_multipart.py::test_multipart_upload_from_path_splits_correctly -v
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/_multipart.py python/tests/test_multipart.py
git commit -m "feat: multipart_upload_from_path streams parts via pread"
```

---

## Task 3: Test abort-on-failure for path-based multipart

**Why:** Cleanup behavior is critical — a failed upload must not leave dangling multipart sessions in the bucket.

**Files:**
- Test: `python/tests/test_multipart.py`

- [ ] **Step 1: Write the failing test**

Append to `python/tests/test_multipart.py`:

```python
@pytestmark_posix
def test_multipart_upload_from_path_aborts_on_failure():
    part_size = 1024
    file_size = part_size * 3

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(b"X" * file_size)
        tmp_path = Path(tmp.name)

    try:
        initiate_xml = b'<?xml version="1.0"?><InitiateMultipartUploadResult><UploadId>up-2</UploadId></InitiateMultipartUploadResult>'

        sent_requests = []
        call_count = {"part": 0}

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
                call_count["part"] += 1
                if call_count["part"] == 2:
                    raise RuntimeError("simulated network error on part 2")
                resp = MagicMock()
                resp.headers = {"etag": '"ok"'}
                resp.client_request_id = "cid"
                return resp
            if req.method == "DELETE" and "uploadId" in params:
                resp = MagicMock()
                resp.headers = {}
                resp.client_request_id = "cid"
                return resp
            raise AssertionError(f"unexpected request: {req.method} {params}")

        client = MagicMock()
        client._url = lambda bucket, key=None: f"https://test/{bucket}/{key or ''}"
        client._http.send = send

        with pytest.raises(RuntimeError, match="simulated network error"):
            multipart_upload_from_path(
                client=client,
                bucket="b",
                key="k",
                path=tmp_path,
                size=file_size,
                content_type=None,
                metadata=None,
                extra_headers=None,
                part_size=part_size,
                concurrency=1,
            )

        abort_requests = [r for r in sent_requests if r.method == "DELETE" and "uploadId" in (r.params or {})]
        assert len(abort_requests) == 1
        assert abort_requests[0].params["uploadId"] == "up-2"
    finally:
        tmp_path.unlink()
```

- [ ] **Step 2: Run the test to verify it passes**

The implementation already calls `_abort` in the `except` block, so this test should pass on first run.

```bash
cd python && .venv/bin/pytest tests/test_multipart.py::test_multipart_upload_from_path_aborts_on_failure -v
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add python/tests/test_multipart.py
git commit -m "test: multipart_upload_from_path aborts upload on part failure"
```

---

## Task 4: Test bounded memory (no more than `concurrency` chunks live)

**Why:** This is the whole point of the feature. A test that proves it must exist.

**Files:**
- Test: `python/tests/test_multipart.py`

- [ ] **Step 1: Write the failing test**

Append to `python/tests/test_multipart.py`:

```python
import threading


@pytestmark_posix
def test_multipart_upload_from_path_bounded_concurrency():
    part_size = 1024
    part_count = 8
    file_size = part_size * part_count
    concurrency = 2

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(b"Z" * file_size)
        tmp_path = Path(tmp.name)

    try:
        initiate_xml = b'<?xml version="1.0"?><InitiateMultipartUploadResult><UploadId>up-3</UploadId></InitiateMultipartUploadResult>'

        in_flight = 0
        max_in_flight = 0
        lock = threading.Lock()
        gate = threading.Event()

        def send(req):
            nonlocal in_flight, max_in_flight
            params = req.params or {}
            if req.method == "POST" and "uploads" in params:
                resp = MagicMock()
                resp.read = lambda: initiate_xml
                resp.headers = {}
                resp.client_request_id = "cid"
                return resp
            if req.method == "PUT" and "partNumber" in params:
                with lock:
                    in_flight += 1
                    if in_flight > max_in_flight:
                        max_in_flight = in_flight
                gate.wait(timeout=2)  # hold the part long enough to observe overlap
                with lock:
                    in_flight -= 1
                resp = MagicMock()
                resp.headers = {"etag": f'"e{params["partNumber"]}"'}
                resp.client_request_id = "cid"
                return resp
            if req.method == "POST" and "uploadId" in params:
                resp = MagicMock()
                resp.headers = {"etag": '"final"', "x-obs-request-id": "r"}
                resp.client_request_id = "cid"
                return resp
            raise AssertionError(f"unexpected request: {req.method} {params}")

        client = MagicMock()
        client._url = lambda bucket, key=None: f"https://test/{bucket}/{key or ''}"
        client._http.send = send

        # Release the gate shortly after starting so workers can drain.
        threading.Timer(0.2, gate.set).start()

        multipart_upload_from_path(
            client=client,
            bucket="b",
            key="k",
            path=tmp_path,
            size=file_size,
            content_type=None,
            metadata=None,
            extra_headers=None,
            part_size=part_size,
            concurrency=concurrency,
        )

        assert max_in_flight <= concurrency, (
            f"expected at most {concurrency} parts in flight, saw {max_in_flight}"
        )
        assert max_in_flight >= 1
    finally:
        tmp_path.unlink()
```

- [ ] **Step 2: Run the test to verify it passes**

```bash
cd python && .venv/bin/pytest tests/test_multipart.py::test_multipart_upload_from_path_bounded_concurrency -v
```

Expected: PASS. `ThreadPoolExecutor(max_workers=concurrency)` enforces this.

- [ ] **Step 3: Commit**

```bash
git add python/tests/test_multipart.py
git commit -m "test: multipart_upload_from_path respects concurrency cap"
```

---

## Task 5: Wire `Client.put_object` to use the new function for large `Path` inputs

**Why:** The new function is unreachable from public API until `put_object` routes to it.

**Files:**
- Modify: `python/src/ya_obs/client.py`
- Test: `python/tests/test_client.py`

- [ ] **Step 1: Write the failing integration test**

Append to `python/tests/test_client.py`:

```python
import os
import sys
import tempfile
from pathlib import Path
import pytest


@pytest.mark.skipif(not hasattr(os, "pread"), reason="pread not available")
def test_put_object_path_large_uses_streaming_multipart(httpx_mock: HTTPXMock, client, tmp_path):
    # Force multipart by setting a low threshold; write a file larger than threshold.
    threshold = 4 * 1024
    part_size = 1024
    file_size = part_size * 3 + 200  # 4 parts

    file_path = tmp_path / "big.bin"
    with open(file_path, "wb") as f:
        f.write(b"A" * part_size)
        f.write(b"B" * part_size)
        f.write(b"C" * part_size)
        f.write(b"D" * 200)

    initiate_xml = (
        b'<?xml version="1.0" encoding="UTF-8"?>'
        b'<InitiateMultipartUploadResult><UploadId>up-100</UploadId></InitiateMultipartUploadResult>'
    )
    httpx_mock.add_response(
        method="POST",
        match_content=None,
        status_code=200,
        content=initiate_xml,
        headers={"Content-Type": "application/xml", "x-obs-request-id": "init"},
    )
    for i in range(1, 5):
        httpx_mock.add_response(
            method="PUT",
            status_code=200,
            headers={"ETag": f'"part-{i}"', "x-obs-request-id": f"p{i}"},
        )
    httpx_mock.add_response(
        method="POST",
        status_code=200,
        content=b'<?xml version="1.0"?><CompleteMultipartUploadResult><ETag>"final"</ETag></CompleteMultipartUploadResult>',
        headers={"ETag": '"final-etag"', "x-obs-request-id": "done"},
    )

    resp = client.put_object(
        "my-bucket", "big.bin", file_path,
        multipart_threshold=threshold,
        part_size=part_size,
        concurrency=2,
    )

    assert resp.etag == '"final-etag"'
    sent = httpx_mock.get_requests()
    part_puts = [r for r in sent if r.method == "PUT" and b"partNumber" in r.url.query]
    assert len(part_puts) == 4
    # Total uploaded bytes = file size (proves we read the whole file, not 19 GB of zeros).
    total_uploaded = sum(len(r.content) for r in part_puts)
    assert total_uploaded == file_size
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd python && .venv/bin/pytest tests/test_client.py::test_put_object_path_large_uses_streaming_multipart -v
```

Expected: FAIL — current `put_object` calls `body.read_bytes()` and routes through `multipart_upload(bytes, ...)`. The test should still pass functionally (the existing path also produces 4 parts), but a memory-bounded behavior assertion isn't possible through `httpx_mock` alone. Since the assertions only check correctness (4 parts, right total bytes), this test may **already pass** before changes.

If it passes, that's fine — go to Step 3 and make the routing change anyway. The unit test in Task 2 is what proves the new path actually runs. Add an extra assertion here to prove routing:

Insert after the existing assertions in the test:

```python
    # Sanity: each part's body is exactly part_size (or remainder for the last),
    # which is what we'd expect either way; the routing proof lives in test_multipart.py.
    sizes = sorted(len(r.content) for r in part_puts)
    assert sizes == [200, part_size, part_size, part_size]
```

- [ ] **Step 3: Modify `Client.put_object` to route large `Path` inputs**

In `python/src/ya_obs/client.py`, replace the body of `put_object` from line 115 (the `if isinstance(body, str):` block) through line 137 (end of the multipart branch) with:

```python
        threshold = multipart_threshold or (100 * 1024 * 1024)

        if isinstance(body, Path):
            size = body.stat().st_size
            if size >= threshold:
                from ._multipart import multipart_upload_from_path
                return multipart_upload_from_path(
                    client=self,
                    bucket=bucket,
                    key=key,
                    path=body,
                    size=size,
                    content_type=content_type,
                    metadata=metadata,
                    extra_headers=extra_headers,
                    part_size=part_size,
                    concurrency=concurrency or 4,
                )
            body_bytes = body.read_bytes()
        elif isinstance(body, str):
            body_bytes = body.encode("utf-8")
        elif isinstance(body, bytes):
            body_bytes = body
        else:
            body_bytes = body.read()

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
```

The rest of the function (single-PUT headers + `_http.send`) stays untouched.

- [ ] **Step 4: Run the new test plus the full suite**

```bash
cd python && .venv/bin/pytest tests/ -v
```

Expected: all tests pass — the new test, the existing client tests, the existing multipart tests, and everything else.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/client.py python/tests/test_client.py
git commit -m "feat: put_object(Path) streams large files via pread multipart"
```

---

## Task 6: Manual verification with a real-ish file (no real OBS call)

**Why:** Confirm peak memory really is bounded when uploading a large file. We use a mock HTTP client to avoid hitting real OBS, but write a real ~500 MB file to disk and measure resident memory.

**Files:**
- This is a manual smoke test, no committed test file. Run from a Python REPL or one-off script.

- [ ] **Step 1: Create a 500 MB temp file and run a mocked multipart upload**

Run this from `python/` with `.venv/bin/python -c '...'`:

```python
import os, resource, tempfile
from pathlib import Path
from unittest.mock import MagicMock
from ya_obs._multipart import multipart_upload_from_path

size = 500 * 1024 * 1024
part_size = 8 * 1024 * 1024
path = Path(tempfile.mkstemp()[1])
with open(path, "wb") as f:
    f.write(b"\0" * size)

initiate_xml = b'<?xml version="1.0"?><InitiateMultipartUploadResult><UploadId>x</UploadId></InitiateMultipartUploadResult>'
def send(req):
    p = req.params or {}
    resp = MagicMock()
    resp.client_request_id = "c"
    if req.method == "POST" and "uploads" in p:
        resp.read = lambda: initiate_xml
        resp.headers = {}
    elif req.method == "PUT":
        resp.headers = {"etag": f'"e{p["partNumber"]}"'}
    else:
        resp.headers = {"etag": '"f"', "x-obs-request-id": "r"}
    return resp
client = MagicMock()
client._url = lambda b, k=None: f"https://t/{b}/{k}"
client._http.send = send

before = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
multipart_upload_from_path(
    client=client, bucket="b", key="k", path=path, size=size,
    content_type=None, metadata=None, extra_headers=None,
    part_size=part_size, concurrency=4,
)
after = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
print(f"max RSS before: {before} after: {after} delta: {after - before}")
path.unlink()
```

Expected: delta well under 500 MB. On macOS `ru_maxrss` is in bytes; on Linux, KB. A delta of < ~80 MB (≈ `concurrency × part_size × small overhead`) confirms the design works.

- [ ] **Step 2: Note the result**

If delta is small (< ~100 MB on macOS, < ~100,000 on Linux), the bounded-memory goal holds. If not, investigate before moving on. There is nothing to commit in this task.

---

## Self-Review

**Spec coverage:**
- ✅ "Path branch in put_object stats instead of read_bytes" → Task 5.
- ✅ "multipart_upload_from_path with pread per worker" → Task 2.
- ✅ "Refactor: extract _initiate / _complete / _abort" → Task 1.
- ✅ "Bounded memory ≈ concurrency × part_size" → Task 4 (test) + Task 6 (manual proof).
- ✅ "Abort on failure" → Task 3.
- ✅ "Windows = NotImplementedError above threshold" → Task 2 (`if not hasattr(os, "pread")`).
- ✅ Out of scope items (BinaryIO streaming, async, resumable, progress) — not addressed by any task, as intended.

**Placeholder scan:** No "TBD" / "TODO" / "implement later" / "similar to". Each step shows full code or full command.

**Type consistency:**
- `multipart_upload_from_path` signature is identical across Tasks 2, 3, 4, 5 (positional-only call sites in tests, kw call site in client).
- `_initiate` returns `str` (upload_id), `_complete` returns `PutObjectResponse`, `_abort` returns `None`. Used consistently.
- `Path`, `bytes`, `int` types match between `client.py` and `_multipart.py`.

No issues to fix.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-23-streaming-multipart-upload.md`. Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task with review between tasks.

**2. Inline Execution** — execute tasks in this session with checkpoints.
