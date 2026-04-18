# ya-obs-sdk — Design Specification

**Date:** 2026-04-17
**Status:** Draft for review
**Author:** Sean

## Goal

Build a clean, minimal multi-language SDK for Huawei Cloud OBS (Object Storage Service). Start as a personal-use SDK; keep the architecture open to becoming a production-grade public release later.

## Strategy

**Strategy 3 — Reference implementation + ports.** Build each language SDK natively/idiomatically. Share a written specification and a canonical **test-vector suite** that every SDK must pass. Test vectors act as the cross-language drift firewall.

**Language order:** Python → TypeScript → Rust.

**Rationale:** Python first for immediate personal use. TypeScript second for broader reach (Bun-developed, Node-compatible, Web-standard APIs only in library code). Rust last for performance and binary distribution.

## Repository Layout

```
ya-obs-sdk/
├── README.md
├── docs/
│   ├── spec/                    # Language-agnostic spec (source of truth)
│   │   ├── api.md               # Operations, parameters, return shapes
│   │   ├── auth.md              # Signing algorithm (V4)
│   │   ├── errors.md            # Error codes, retry semantics
│   │   └── multipart.md         # Chunking/multipart upload behavior
│   └── superpowers/specs/       # Design docs (this file)
├── test-vectors/                # Shared fixtures — drift firewall
│   ├── auth/                    # Signing inputs → expected signatures
│   ├── xml/                     # Request/response XML samples
│   └── multipart/               # Multipart scenarios
├── python/                      # Python SDK (first)
│   ├── pyproject.toml
│   ├── src/ya_obs/
│   └── tests/
├── typescript/                  # TS SDK (second)
│   ├── package.json
│   ├── src/
│   └── tests/
└── rust/                        # Rust SDK (third)
    ├── Cargo.toml
    ├── src/
    └── tests/
```

### Versioning

**Independent** per-package versioning. Each package has its own version, CHANGELOG, and release cadence. The monorepo is an implementation detail; users of `ya-obs` on PyPI don't see npm or crates.io versions.

## v0.1 Scope (Python)

**In scope:**

| Tier | Operations |
|------|------------|
| 1 — Object basics | `put_object`, `get_object`, `head_object`, `delete_object`, `list_objects` |
| 3 — Large file / perf | Multipart upload (`initiate`/`upload_part`/`complete`/`abort`/`list_parts`), server-side copy, range downloads |
| 4 — Metadata & utilities | User metadata (`x-obs-meta-*`), common system metadata (Content-Type, Cache-Control, etc.), presigned URLs |

**Out of scope for v0.1:** bucket management (Tier 2), lifecycle rules, ACLs, policies, CORS, versioning, tags, website hosting, event notifications, replication, SSE-KMS, Select, inventory.

## Test Vectors & Conformance

Test vectors are the **executable contract**. Stored as JSON in `test-vectors/` at the repo root. Each language's test suite loads and asserts against them.

**Vector categories:**
- **Pure-function vectors** (auth): `input → expected` for both V4 and V2 signing. V4 vectors include intermediate values (`canonical_request`, `string_to_sign`, `signing_key`, `signature`); V2 vectors include `string_to_sign` and `signature`. Both cover Authorization-header and presigned-URL forms.
- **Codec vectors** (XML): serialize/parse round-trip and fixed-output assertions.
- **Scenario vectors** (multipart): multi-step HTTP interaction recordings.
- **Error vectors**: raw HTTP response → expected exception type and fields.

**Format:** Each vector is a versioned JSON file (`"version": 1`) containing `name`, `description`, `input`, `expected`. Binary data encoded as base64 or hex.

**Conformance suite:** A compliance matrix per-SDK (auth / xml / multipart / retries / errors). Partial compliance is allowed and reported honestly.

## Authentication

### Two signing schemes

**V4 (default):** AWS SigV4-compatible. Produces `Authorization: AWS4-HMAC-SHA256 Credential=..., SignedHeaders=..., Signature=...`. Well-specified, preferred for new Huawei Cloud regions, covers presigned URLs cleanly via query-string signing.

**OBS V2 (custom Huawei):** Huawei's legacy scheme. HMAC-SHA1 over a canonical string:

```
StringToSign = HTTP-Method + "\n"
             + Content-MD5 + "\n"
             + Content-Type + "\n"
             + Date + "\n"
             + CanonicalizedOBSHeaders
             + CanonicalizedResource
```

Produces `Authorization: OBS <AccessKeyID>:<Signature>`. Presigned URLs use query-string form: `AccessKeyId`, `Expires` (Unix timestamp), `Signature` as query params.

**Why both:** V4 is the default and preferred. V2 is required for interop with Huawei's own tooling (OBS Browser+), older SDKs, and some private OBS deployments that have not migrated.

**Default:** V4. User selects V2 via `Client(signing_version="v2")` or per-call override.

### Credential resolution

Two-level resolution:
1. Explicit kwargs to `Client(access_key=..., secret_key=...)`.
2. Environment variables: `HUAWEICLOUD_SDK_AK`, `HUAWEICLOUD_SDK_SK`.

No filesystem config files, no instance metadata fallback in v0.1.

### Signer architecture

`Signer` protocol:

```python
class Signer(Protocol):
    def sign(self, request: Request) -> Request: ...
```

Two implementations shipped: `SignerV4` and `SignerV2`. Both implement the `Signer` protocol, separable from HTTP so each can be tested against its own test vectors without network I/O.

### Presigned URLs (v0.1)

- **V4:** query-string signing (`X-Amz-Algorithm`, `X-Amz-Credential`, `X-Amz-Signature`, etc.). Expiry via `X-Amz-Expires` (seconds).
- **V2:** query-string signing (`AccessKeyId`, `Expires` as Unix timestamp, `Signature`).

API: `client.presign_get_object(bucket, key, expires=3600) -> str`. Uses the client's configured signing version.

## HTTP Layer

- **Library:** `httpx` (sync + async from one codebase, typed, HTTP/2, streaming).
- **Clients:** `ya_obs.Client` (sync) and `ya_obs.AsyncClient` (async), both thin wrappers over shared signer/codec/URL-builder.
- **Connection pooling:** always on; closed via context manager or explicit `.close()`.
- **`httpx` is not exposed** in public API.

## Client Configuration

### Construction

Direct constructor with common params:

```python
client = Client(
    access_key="...",              # or env HUAWEICLOUD_SDK_AK
    secret_key="...",              # or env HUAWEICLOUD_SDK_SK
    region="cn-north-4",           # OR endpoint=...
    endpoint=None,                 # full URL, overrides region if given
    addressing_style="auto",       # "auto" | "virtual" | "path"
    timeout=Timeout(connect=10, read=60, total=None),
    retry_policy=RetryPolicy(...),
    on_retry=None,                 # optional callback
)
```

### Region / endpoint

- `region` → SDK builds `https://obs.<region>.myhuaweicloud.com`.
- `endpoint` → used as-is (overrides region).
- Either required.

### Addressing style

Default `"auto"`:
- Virtual-hosted (`<bucket>.obs.<region>.myhuaweicloud.com`) when endpoint is a known OBS domain **and** bucket name is DNS-safe (no dots in HTTPS).
- Path-style (`obs.<region>.myhuaweicloud.com/<bucket>`) otherwise.

Users can force with `addressing_style="virtual"` or `"path"`.

### Request IDs

- **Client-generated:** UUID per attempt, sent as `x-ya-obs-client-id` header. Logged on every attempt; attached to exceptions.
- **Server-returned:** `x-obs-request-id` captured from response; attached to responses and exceptions.

### Per-call overrides

Only `timeout` and `retries` overridable per-call. Credentials, endpoint, region fixed at client level.

## Object API

### put_object

```python
client.put_object(
    bucket: str,
    key: str,
    body: bytes | str | BinaryIO | Path | Iterator[bytes],
    *,
    content_type: str | None = None,
    content_encoding: str | None = None,
    cache_control: str | None = None,
    content_disposition: str | None = None,
    expires: datetime | None = None,
    metadata: dict[str, str] | None = None,     # x-obs-meta-*
    extra_headers: dict[str, str] | None = None,  # escape hatch
    # Multipart tuning
    multipart_threshold: int | None = None,     # override default
    part_size: int | None = None,
    concurrency: int | None = None,
) -> PutObjectResponse
```

**Multipart auto-trigger rules:**
- `body` is a stream of unknown size → always multipart.
- `body` has known size ≥ `multipart_threshold` (default 100 MB) → multipart.
- Otherwise → single PUT.

**Part sizing (adaptive):** `part_size = max(8 MB, ceil(total_size / 9000))` when total size is known. Default 8 MB for streams.

**Concurrency:** default 4 parts in flight. Configurable per-call.

**Failure handling:** retry each failed part up to `retry_policy.max_attempts` times. If retries exhausted, call `abort_multipart_upload` then raise.

### get_object

```python
client.get_object(
    bucket: str,
    key: str,
    *,
    range: tuple[int, int | None] | None = None,  # (start, end_inclusive or None)
    if_match: str | None = None,
    if_none_match: str | None = None,
) -> GetObjectResponse
```

`GetObjectResponse` has streaming body by default: `resp.iter_bytes()`, `resp.iter_lines()`, `resp.read()` (buffers all), `resp.save_to(path)`.

Plus typed fields: `content_type`, `content_length`, `etag`, `last_modified` (datetime), `metadata` (dict), `cache_control`, `content_encoding`, `content_disposition`.

### head_object, delete_object, list_objects

`head_object` returns same fields as `get_object` response minus body.
`delete_object` semantics (idempotency on missing key) are listed in Open Questions; implementation deferred until confirmed against the live OBS API.
`list_objects` returns an iterator that handles pagination transparently; also exposes `list_objects_page(...)` for manual pagination.

### copy_object

```python
client.copy_object(
    src_bucket: str, src_key: str,
    dst_bucket: str, dst_key: str,
    *,
    metadata_directive: "COPY" | "REPLACE" = "COPY",
    metadata: dict[str, str] | None = None,
    # system metadata overrides when REPLACE...
) -> CopyObjectResponse
```

Server-side copy. No data flows through the client.

## Response Types

Typed dataclasses. Example:

```python
@dataclass
class PutObjectResponse:
    etag: str
    version_id: str | None
    request_id: str
    client_request_id: str

@dataclass
class GetObjectResponse:
    body: StreamingBody            # .iter_bytes(), .read(), .save_to()
    content_type: str | None
    content_length: int | None
    etag: str
    last_modified: datetime
    metadata: dict[str, str]
    cache_control: str | None
    content_encoding: str | None
    content_disposition: str | None
    request_id: str
    client_request_id: str
```

## Error Handling

### Hierarchy

```
ObsError(Exception)
├── ClientError              # 4xx
│   ├── NoSuchKey            # 404 NoSuchKey
│   ├── NoSuchBucket         # 404 NoSuchBucket
│   └── AccessDenied         # 403 AccessDenied
└── ServerError              # 5xx
```

All exceptions carry: `code`, `message`, `status`, `request_id`, `host_id`, `client_request_id`.

Additional named subclasses only added when there's clear user value in catching by type. Users can always inspect `e.code` for OBS-specific codes.

### Retry policy

Retry on:
- Connection errors (timeout, refused, reset).
- 5xx server errors.
- 408 `RequestTimeout`, 429 `TooManyRequests` / `SlowDown`.

**Default:** exponential backoff with full jitter, base 0.5 s, max 30 s, 3 attempts total (1 initial + 2 retries).

Honor `Retry-After` header when present (overrides computed delay).

Configurable via `RetryPolicy(max_attempts, base_delay, max_delay, jitter)`.

### Observability

- Retries logged at `WARNING` via `logging.getLogger("ya_obs")`.
- Optional `on_retry: Callable[[RetryEvent], None]` client-level callback for programmatic access.

### Timeouts

Three-level:
- `connect` (default 10 s)
- `read` (default 60 s)
- `total` (default `None`; user-configurable operation-wide cap including retries)

## Non-Goals

- Plugins / middleware / extension points beyond the `Signer` protocol.
- Full AWS-style credential chain (config files, instance metadata).
- STS tokens / temporary credentials (defer to post-v0.1).
- Bucket management operations.
- High-level convenience wrappers (sync/copy-tree, CLI) — out of SDK scope.
- Implementing anything in Tier 5 (ACL, lifecycle, CORS, etc.) in v0.1.

## Open Questions

1. **`delete_object` semantics:** confirm with OBS docs whether DELETE on a missing key returns 204 (like S3) or 404. This affects idempotency claims. → Verify before implementing.
2. **Region discovery:** should `list_objects` on a bucket in the wrong region auto-redirect (like S3 `301 PermanentRedirect` with `x-amz-bucket-region`)? → Defer decision until first hit.
3. **Checksums:** OBS supports `Content-MD5` on uploads and `x-obs-checksum-*` headers. v0.1 includes `Content-MD5` for `put_object` when body is known-size bytes; multipart checksums deferred.

## Success Criteria for v0.1

- All Tier 1 + Tier 3 + Tier 4 operations implemented in Python.
- Conformance suite: 100% of auth vectors, 100% of XML vectors, 100% of error vectors pass.
- Multipart upload reliably handles a 5 GB file over a flaky connection.
- Presigned GET URLs work in `curl` and browser.
- Unit tests + conformance tests green on CI.
- README with install + quickstart + API reference.
