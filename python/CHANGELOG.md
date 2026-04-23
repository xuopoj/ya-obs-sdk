# Changelog

## 0.1.1 (2026-04-23)

### Added
- Streaming multipart uploads from disk: `Client.put_object(body=Path(...))` now reads parts via `os.pread` from a per-worker file descriptor instead of loading the whole file into memory. Peak memory is bounded to ~`concurrency × part_size` (default ~32 MB), enabling uploads of arbitrarily large files. POSIX only; raises `NotImplementedError` on Windows for files at or above the multipart threshold.

## 0.1.0 (2026-04-18)

### Added
- `Client` and `AsyncClient` with `put_object`, `get_object`, `head_object`, `delete_object`, `list_objects`, `list_objects_page`, `copy_object`, `presign_get_object`
- Automatic multipart upload for objects ≥ 100 MB with adaptive part sizing and configurable concurrency
- V4 (AWS SigV4-compatible) and V2 (OBS custom HMAC-SHA1) signing
- Streaming response body with `.iter_bytes()`, `.read()`, `.save_to()`
- User metadata support (`x-obs-meta-*`)
- Exponential backoff retry with jitter
- Typed response dataclasses
- Conformance test vectors for auth (V2/V4), XML codec, and error parsing
