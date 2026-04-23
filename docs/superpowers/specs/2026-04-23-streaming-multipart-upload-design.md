# Streaming multipart upload from disk

## Problem

`Client.put_object` currently materializes the entire upload body in memory:

- `client.py:118` — `Path` input calls `body.read_bytes()`, slurping the whole file.
- `client.py:122` — `BinaryIO` input calls `body.read()`, slurping the whole stream.
- `_multipart.py:33,60` — `multipart_upload(body: bytes, ...)` then calls `split_into_parts(data: bytes, ...)`, which holds every part as a separate `bytes` slice for the lifetime of the upload.

For a 19 GB file (e.g. COCO `train2017.zip`) this means ~19 GB of resident memory plus a second ~19 GB while parts are sliced off (until the original `bytes` is dropped). The single-PUT path under `multipart_threshold` is acceptable as-is (default 100 MB).

## Goal

Allow `put_object` to upload arbitrarily large files from disk with **bounded memory** ≈ `concurrency × part_size` (default ~32 MB), without changing behavior for callers passing `bytes` / `str`.

Non-goals:
- Streaming uploads from non-seekable `BinaryIO` (network sockets, pipes). Per-part retry requires re-reading a part on failure, which non-seekable streams can't do. YAGNI until requested.
- Async client streaming. Add later if needed; this spec is sync-only.
- Zero-copy via `sendfile` / `splice` / kTLS. Won't work through TLS + httpx; the SigV4 part-hash also forces bytes into userspace anyway.

## Design

### Public API change

`Client.put_object` already accepts `body: bytes | str | BinaryIO | Path`. Only the `Path` branch changes semantics: instead of `read_bytes()`, we stat the file and pass `(path, size)` through to multipart.

```python
elif isinstance(body, Path):
    size = body.stat().st_size
    threshold = multipart_threshold or (100 * 1024 * 1024)
    if size >= threshold:
        return multipart_upload_from_path(
            client=self, bucket=bucket, key=key,
            path=body, size=size,
            content_type=..., metadata=..., extra_headers=...,
            part_size=part_size, concurrency=concurrency or 4,
        )
    body_bytes = body.read_bytes()  # small file, fine to buffer
```

`bytes` and `str` paths are unchanged. `BinaryIO` path is unchanged (still buffers; will document the limitation). No new public parameters.

### Multipart from path

Add a new function `multipart_upload_from_path` in `_multipart.py`. Keep the existing `multipart_upload(body: bytes, ...)` for the in-memory path — they share the init / complete / abort logic but differ in how parts are produced.

```python
def multipart_upload_from_path(
    client, bucket, key, path: Path, size: int,
    content_type, metadata, extra_headers,
    part_size: int | None, concurrency: int,
) -> PutObjectResponse:
    actual_part_size = part_size or compute_part_size(size)
    part_count = math.ceil(size / actual_part_size)

    upload_id = _initiate(client, bucket, key, content_type, metadata, extra_headers)

    def upload_part(part_number: int) -> tuple[int, str]:
        offset = (part_number - 1) * actual_part_size
        length = min(actual_part_size, size - offset)
        with open(path, "rb") as f:
            chunk = os.pread(f.fileno(), length, offset)  # POSIX; thread-safe per fd
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
            for f in as_completed(futures):
                completed_parts.append(f.result())
    except Exception:
        _abort(client, bucket, key, upload_id)
        raise

    return _complete(client, bucket, key, upload_id, completed_parts)
```

Key properties:
- **Each worker opens its own fd.** Avoids any shared-state concerns with `pread`. `os.pread` is also thread-safe on a shared fd, but per-worker fds are simpler and cost ~nothing.
- **Peak memory = `concurrency × part_size`.** At any moment at most `concurrency` chunks are in flight; once `httpx.send` returns, the chunk is droppable.
- **`pread` over `seek + read`.** Single syscall, no shared file pointer, no thread-locking dance.
- **Retry-safe.** The existing `RetryLoop` re-invokes `_attempt`, which re-runs the closure passed to `client._http.send`. Our closure captures `chunk` (already in memory for the duration of the part upload), so a retry of a single part doesn't re-read from disk — but it also doesn't need to.

### Refactor: extract shared helpers

Pull the init / complete / abort blocks out of the existing `multipart_upload` into private helpers (`_initiate`, `_complete`, `_abort`). Both the bytes path and the path-based path call them. This is bounded refactor justified by the new caller — not gratuitous cleanup.

After the refactor, `multipart_upload(body: bytes, ...)` shrinks to: init → split → upload chunks → complete (with abort on error). The path version is the same shape but produces chunks via `pread` instead of slicing.

### Platform note

`os.pread` is POSIX-only (Linux, macOS). Windows lacks it. Implementation strategy:

- Detect `hasattr(os, "pread")` at import.
- On POSIX: streaming path-based multipart as designed.
- On Windows, files **below** the multipart threshold: still fine (small enough to buffer).
- On Windows, files **at or above** the threshold: raise a clear `NotImplementedError` ("path-based multipart upload of files >= multipart_threshold is not supported on Windows; pass `bytes` or upload from a POSIX host"). Falling back to `read_bytes()` would silently allocate 19 GB and defeat the spec's purpose.

Acceptable because the SDK has no Windows CI and the immediate motivating use case (COCO upload) is on macOS. Windows support can be added later via `ReadFile` with `OVERLAPPED` if a user needs it.

## Testing

Three new tests in `tests/test_multipart.py`:

1. **Functional, mocked HTTP.** Write a temp file of `part_size * 3 + 1` bytes, mock the HTTP client to record each part's body, run `put_object(Path)`, assert: 4 parts, each with the right offset/length, `CompleteMultipartUpload` includes all etags in order.
2. **Bounded memory.** Same setup, but assert that at no point during the upload do more than `concurrency` chunks exist simultaneously. Implementable by having the mocked `send` block on a semaphore and counting live chunks via `weakref` or by instrumenting the `upload_part` closure.
3. **Abort on failure.** Mock the HTTP client to fail on part 2; assert that `DELETE ?uploadId=...` is sent and the original error propagates.

Skip-on-Windows marker for tests that exercise `pread`.

No integration test against real OBS in this spec — that's a separate manual verification step (the COCO upload itself).

## Out of scope (explicit)

- `BinaryIO` streaming — needs a different design for non-seekable streams.
- Async path — `AsyncClient.put_object` doesn't even have multipart yet (`client.py:350` only does single PUT). Separate spec.
- Resumable uploads (persist `upload_id` + completed parts to disk, resume on next run). Useful for 19 GB uploads over flaky links, but a separate feature.
- Progress callbacks. Separate feature.

## Migration / compatibility

No breaking changes. Existing callers passing `bytes` / `str` / `BinaryIO` see identical behavior. Callers passing `Path` get bounded memory for free; the public signature is unchanged.
