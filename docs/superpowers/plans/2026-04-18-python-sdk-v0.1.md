# ya-obs Python SDK v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Python ya-obs SDK v0.1 covering object basics, multipart upload, metadata, and presigned URLs against Huawei Cloud OBS.

**Architecture:** Layered — signer (pure functions, no I/O) → HTTP core (request/response models, URL builder, retry) → object API (thin methods that build requests and parse responses) → multipart orchestrator (concurrency layer on top of object API). Each layer is independently testable.

**Tech Stack:** Python 3.11+, httpx, pytest, uv (package manager)

---

## File Map

```
python/
├── pyproject.toml
├── src/ya_obs/
│   ├── __init__.py           # public re-exports
│   ├── _models.py            # Request, Response, Timeout, RetryPolicy, RetryEvent dataclasses
│   ├── _errors.py            # ObsError hierarchy
│   ├── _signer_v4.py         # SignerV4
│   ├── _signer_v2.py         # SignerV2
│   ├── _xml.py               # XML codec (serialize/parse OBS XML)
│   ├── _url.py               # URL builder + addressing style logic
│   ├── _retry.py             # retry loop with backoff + jitter
│   ├── _http.py              # httpx wrapper (sync + async), injects signer/retry/request-id
│   ├── _streaming.py         # StreamingBody wrapper
│   ├── _multipart.py         # multipart orchestrator (adaptive part size, concurrency)
│   ├── _responses.py         # PutObjectResponse, GetObjectResponse, etc. dataclasses
│   └── client.py             # Client + AsyncClient (public API surface)
└── tests/
    ├── conftest.py
    ├── vectors/              # loads test-vectors/ from repo root
    │   ├── test_auth_vectors.py
    │   ├── test_xml_vectors.py
    │   └── test_error_vectors.py
    ├── test_signer_v4.py
    ├── test_signer_v2.py
    ├── test_url.py
    ├── test_retry.py
    ├── test_xml.py
    ├── test_streaming.py
    ├── test_multipart.py
    └── test_client.py        # integration-style tests using httpx mock transport
```

```
test-vectors/
├── auth/
│   ├── v4_header_basic.json
│   ├── v4_header_with_query.json
│   ├── v4_presign_basic.json
│   ├── v2_header_basic.json
│   ├── v2_header_content_md5.json
│   └── v2_presign_basic.json
├── xml/
│   ├── list_objects_response.json
│   ├── initiate_multipart_response.json
│   ├── complete_multipart_request.json
│   └── error_response.json
└── errors/
    ├── no_such_key.json
    ├── no_such_bucket.json
    └── access_denied.json
```

---

## Task 1: Project scaffold

**Files:**
- Create: `python/pyproject.toml`
- Create: `python/src/ya_obs/__init__.py`
- Create: `python/tests/conftest.py`
- Create: `.gitignore`
- Create: `README.md`

- [ ] **Step 1: Initialize project structure**

```bash
cd /path/to/ya-obs-sdk
mkdir -p python/src/ya_obs python/tests test-vectors/auth test-vectors/xml test-vectors/errors
touch python/src/ya_obs/__init__.py python/tests/conftest.py
```

- [ ] **Step 2: Write pyproject.toml**

Create `python/pyproject.toml`:

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "ya-obs"
version = "0.1.0"
description = "A clean Python SDK for Huawei Cloud OBS"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.27",
]

[project.optional-dependencies]
dev = [
    "pytest>=8",
    "pytest-asyncio>=0.23",
    "pytest-httpx>=0.30",
    "anyio[trio]",
]

[tool.hatch.build.targets.wheel]
packages = ["src/ya_obs"]

[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
```

- [ ] **Step 3: Install dependencies**

```bash
cd python
uv venv && uv pip install -e ".[dev]"
```

Expected: virtualenv created, all packages installed without errors.

- [ ] **Step 4: Write minimal __init__.py**

`python/src/ya_obs/__init__.py`:

```python
__version__ = "0.1.0"
```

- [ ] **Step 5: Write conftest.py**

`python/tests/conftest.py`:

```python
import pathlib
import json

VECTORS_DIR = pathlib.Path(__file__).parent.parent.parent / "test-vectors"

def load_vector(category: str, name: str) -> dict:
    return json.loads((VECTORS_DIR / category / f"{name}.json").read_text(encoding="utf-8"))
```

- [ ] **Step 6: Write .gitignore**

`.gitignore`:

```
__pycache__/
*.py[cod]
.venv/
dist/
*.egg-info/
.pytest_cache/
.ruff_cache/
node_modules/
target/
```

- [ ] **Step 7: Verify pytest runs (zero tests, no errors)**

```bash
cd python && python -m pytest --tb=short
```

Expected: `no tests ran` — zero collection errors.

- [ ] **Step 8: Commit**

```bash
git init
git add .
git commit -m "chore: project scaffold — python SDK + test-vectors structure"
```

---

## Task 2: Core models

**Files:**
- Create: `python/src/ya_obs/_models.py`
- Create: `python/src/ya_obs/_errors.py`

- [ ] **Step 1: Write failing test for models**

`python/tests/test_models.py`:

```python
from ya_obs._models import Request, Timeout, RetryPolicy, RetryEvent
from ya_obs._errors import ObsError, ClientError, ServerError, NoSuchKey, NoSuchBucket, AccessDenied

def test_request_defaults():
    r = Request(method="GET", url="https://example.com/bucket/key")
    assert r.headers == {}
    assert r.body is None
    assert r.params == {}

def test_timeout_defaults():
    t = Timeout()
    assert t.connect == 10.0
    assert t.read == 60.0
    assert t.total is None

def test_retry_policy_defaults():
    rp = RetryPolicy()
    assert rp.max_attempts == 3
    assert rp.base_delay == 0.5
    assert rp.max_delay == 30.0
    assert rp.jitter is True

def test_error_hierarchy():
    assert issubclass(ClientError, ObsError)
    assert issubclass(ServerError, ObsError)
    assert issubclass(NoSuchKey, ClientError)
    assert issubclass(NoSuchBucket, ClientError)
    assert issubclass(AccessDenied, ClientError)

def test_obs_error_fields():
    e = NoSuchKey(
        code="NoSuchKey",
        message="The specified key does not exist.",
        status=404,
        request_id="ABC123",
        host_id="HOST456",
        client_request_id="cli-uuid",
    )
    assert e.code == "NoSuchKey"
    assert e.status == 404
    assert str(e) == "NoSuchKey: The specified key does not exist. (request_id=ABC123)"
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_models.py -v
```

Expected: `ModuleNotFoundError` — `_models` not found.

- [ ] **Step 3: Write _models.py**

`python/src/ya_obs/_models.py`:

```python
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Request:
    method: str
    url: str
    headers: dict[str, str] = field(default_factory=dict)
    params: dict[str, str] = field(default_factory=dict)
    body: bytes | None = None


@dataclass
class Timeout:
    connect: float = 10.0
    read: float = 60.0
    total: float | None = None


@dataclass
class RetryPolicy:
    max_attempts: int = 3
    base_delay: float = 0.5
    max_delay: float = 30.0
    jitter: bool = True


@dataclass
class RetryEvent:
    attempt: int
    error: Exception
    delay: float
    request_id: str | None
```

- [ ] **Step 4: Write _errors.py**

`python/src/ya_obs/_errors.py`:

```python
from __future__ import annotations


class ObsError(Exception):
    def __init__(
        self,
        code: str,
        message: str,
        status: int,
        request_id: str = "",
        host_id: str = "",
        client_request_id: str = "",
    ) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status
        self.request_id = request_id
        self.host_id = host_id
        self.client_request_id = client_request_id

    def __str__(self) -> str:
        return f"{self.code}: {self.message} (request_id={self.request_id})"


class ClientError(ObsError):
    pass


class ServerError(ObsError):
    pass


class NoSuchKey(ClientError):
    pass


class NoSuchBucket(ClientError):
    pass


class AccessDenied(ClientError):
    pass


_CODE_TO_CLASS: dict[str, type[ObsError]] = {
    "NoSuchKey": NoSuchKey,
    "NoSuchBucket": NoSuchBucket,
    "AccessDenied": AccessDenied,
}


def make_error(
    code: str,
    message: str,
    status: int,
    request_id: str = "",
    host_id: str = "",
    client_request_id: str = "",
) -> ObsError:
    cls = _CODE_TO_CLASS.get(code)
    if cls is None:
        cls = ClientError if status < 500 else ServerError
    return cls(
        code=code,
        message=message,
        status=status,
        request_id=request_id,
        host_id=host_id,
        client_request_id=client_request_id,
    )
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_models.py -v
```

Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/_models.py python/src/ya_obs/_errors.py python/tests/test_models.py
git commit -m "feat: core models and error hierarchy"
```

---

## Task 3: URL builder

**Files:**
- Create: `python/src/ya_obs/_url.py`
- Create: `python/tests/test_url.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_url.py`:

```python
from ya_obs._url import build_url, is_obs_domain, is_dns_safe_bucket

def test_virtual_hosted_obs_domain():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my-bucket",
        key="photos/cat.jpg",
        addressing_style="auto",
    )
    assert url == "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg"

def test_path_style_forced():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my-bucket",
        key="photos/cat.jpg",
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/my-bucket/photos/cat.jpg"

def test_auto_path_style_for_dotted_bucket():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my.bucket.with.dots",
        key="key.txt",
        addressing_style="auto",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/my.bucket.with.dots/key.txt"

def test_auto_path_style_for_custom_endpoint():
    url = build_url(
        endpoint="http://localhost:9000",
        bucket="testbucket",
        key="file.bin",
        addressing_style="auto",
    )
    assert url == "http://localhost:9000/testbucket/file.bin"

def test_key_encoding():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="mybucket",
        key="path/to/my file & stuff.txt",
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/mybucket/path/to/my%20file%20%26%20stuff.txt"

def test_no_key():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="mybucket",
        key=None,
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/mybucket/"

def test_is_obs_domain():
    assert is_obs_domain("https://obs.cn-north-4.myhuaweicloud.com") is True
    assert is_obs_domain("https://obs.eu-west-0.myhuaweicloud.eu") is True
    assert is_obs_domain("http://localhost:9000") is False
    assert is_obs_domain("https://minio.example.com") is False

def test_is_dns_safe_bucket():
    assert is_dns_safe_bucket("my-bucket") is True
    assert is_dns_safe_bucket("my.bucket") is False
    assert is_dns_safe_bucket("MY-BUCKET") is False  # uppercase invalid in virtual-hosted
    assert is_dns_safe_bucket("a" * 63) is True
    assert is_dns_safe_bucket("a" * 64) is False
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_url.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write _url.py**

`python/src/ya_obs/_url.py`:

```python
from __future__ import annotations
import re
from urllib.parse import quote


_OBS_DOMAIN_RE = re.compile(
    r"^https?://obs\.[a-z0-9-]+\.(myhuaweicloud\.com|myhuaweicloud\.eu)$"
)


def is_obs_domain(endpoint: str) -> bool:
    return bool(_OBS_DOMAIN_RE.match(endpoint.rstrip("/")))


def is_dns_safe_bucket(bucket: str) -> bool:
    return bool(re.match(r"^[a-z0-9][a-z0-9\-]{1,61}[a-z0-9]$", bucket))


def build_url(
    endpoint: str,
    bucket: str,
    key: str | None,
    addressing_style: str = "auto",
) -> str:
    endpoint = endpoint.rstrip("/")
    encoded_key = quote(key or "", safe="/") if key else ""

    use_virtual = (
        addressing_style == "virtual"
        or (
            addressing_style == "auto"
            and is_obs_domain(endpoint)
            and is_dns_safe_bucket(bucket)
        )
    )

    if use_virtual:
        # inject bucket as subdomain
        scheme, rest = endpoint.split("://", 1)
        base = f"{scheme}://{bucket}.{rest}"
        return f"{base}/{encoded_key}"
    else:
        return f"{endpoint}/{bucket}/{encoded_key}"
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_url.py -v
```

Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/_url.py python/tests/test_url.py
git commit -m "feat: URL builder with virtual-hosted and path-style addressing"
```

---

## Task 4: XML codec

**Files:**
- Create: `python/src/ya_obs/_xml.py`
- Create: `python/tests/test_xml.py`
- Create: `test-vectors/xml/list_objects_response.json`
- Create: `test-vectors/xml/initiate_multipart_response.json`
- Create: `test-vectors/xml/complete_multipart_request.json`
- Create: `test-vectors/xml/error_response.json`

- [ ] **Step 1: Write test vectors**

`test-vectors/xml/list_objects_response.json`:

```json
{
  "version": 1,
  "name": "list_objects_response",
  "description": "Parse ListBucketResult XML into object list",
  "input": {
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult xmlns=\"http://obs.myhwclouds.com/doc/2015-06-30/\"><Name>my-bucket</Name><Prefix></Prefix><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated><Contents><Key>photo.jpg</Key><LastModified>2024-01-15T10:30:00.000Z</LastModified><ETag>\"abc123\"</ETag><Size>12345</Size></Contents><Contents><Key>video.mp4</Key><LastModified>2024-02-20T08:00:00.000Z</LastModified><ETag>\"def456\"</ETag><Size>9876543</Size></Contents></ListBucketResult>"
  },
  "expected": {
    "name": "my-bucket",
    "is_truncated": false,
    "next_marker": null,
    "objects": [
      {"key": "photo.jpg", "etag": "\"abc123\"", "size": 12345, "last_modified": "2024-01-15T10:30:00+00:00"},
      {"key": "video.mp4", "etag": "\"def456\"", "size": 9876543, "last_modified": "2024-02-20T08:00:00+00:00"}
    ]
  }
}
```

`test-vectors/xml/initiate_multipart_response.json`:

```json
{
  "version": 1,
  "name": "initiate_multipart_response",
  "description": "Parse InitiateMultipartUploadResult XML",
  "input": {
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><InitiateMultipartUploadResult xmlns=\"http://obs.myhwclouds.com/doc/2015-06-30/\"><Bucket>my-bucket</Bucket><Key>large-file.zip</Key><UploadId>upload-id-12345</UploadId></InitiateMultipartUploadResult>"
  },
  "expected": {
    "bucket": "my-bucket",
    "key": "large-file.zip",
    "upload_id": "upload-id-12345"
  }
}
```

`test-vectors/xml/complete_multipart_request.json`:

```json
{
  "version": 1,
  "name": "complete_multipart_request",
  "description": "Serialize CompleteMultipartUpload XML from part list",
  "input": {
    "parts": [
      {"part_number": 1, "etag": "\"etag-part-1\""},
      {"part_number": 2, "etag": "\"etag-part-2\""},
      {"part_number": 3, "etag": "\"etag-part-3\""}
    ]
  },
  "expected": {
    "xml": "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>\"etag-part-1\"</ETag></Part><Part><PartNumber>2</PartNumber><ETag>\"etag-part-2\"</ETag></Part><Part><PartNumber>3</PartNumber><ETag>\"etag-part-3\"</ETag></Part></CompleteMultipartUpload>"
  }
}
```

`test-vectors/xml/error_response.json`:

```json
{
  "version": 1,
  "name": "error_response",
  "description": "Parse OBS XML error response",
  "input": {
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message><RequestId>req-abc123</RequestId><HostId>host-xyz</HostId></Error>"
  },
  "expected": {
    "code": "NoSuchKey",
    "message": "The specified key does not exist.",
    "request_id": "req-abc123",
    "host_id": "host-xyz"
  }
}
```

- [ ] **Step 2: Write failing tests**

`python/tests/test_xml.py`:

```python
import json
import pathlib
import pytest
from ya_obs._xml import (
    parse_list_objects,
    parse_initiate_multipart,
    parse_error_response,
    serialize_complete_multipart,
)

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "xml"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_parse_list_objects():
    v = load("list_objects_response")
    result = parse_list_objects(v["input"]["xml"])
    assert result["name"] == v["expected"]["name"]
    assert result["is_truncated"] == v["expected"]["is_truncated"]
    assert result["next_marker"] == v["expected"]["next_marker"]
    assert len(result["objects"]) == 2
    assert result["objects"][0]["key"] == "photo.jpg"
    assert result["objects"][0]["size"] == 12345
    assert result["objects"][1]["key"] == "video.mp4"

def test_parse_initiate_multipart():
    v = load("initiate_multipart_response")
    result = parse_initiate_multipart(v["input"]["xml"])
    assert result == v["expected"]

def test_parse_error_response():
    v = load("error_response")
    result = parse_error_response(v["input"]["xml"])
    assert result == v["expected"]

def test_serialize_complete_multipart():
    v = load("complete_multipart_request")
    parts = [(p["part_number"], p["etag"]) for p in v["input"]["parts"]]
    xml = serialize_complete_multipart(parts)
    assert xml == v["expected"]["xml"]
```

- [ ] **Step 3: Run to verify failure**

```bash
cd python && python -m pytest tests/test_xml.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 4: Write _xml.py**

`python/src/ya_obs/_xml.py`:

```python
from __future__ import annotations
import xml.etree.ElementTree as ET
from datetime import datetime, timezone


_NS = "http://obs.myhwclouds.com/doc/2015-06-30/"


def _tag(name: str) -> str:
    return f"{{{_NS}}}{name}"


def _text(el: ET.Element, tag: str, default: str = "") -> str:
    child = el.find(_tag(tag))
    return child.text or default if child is not None else default


def parse_list_objects(xml: str) -> dict:
    root = ET.fromstring(xml)
    objects = []
    for content in root.findall(_tag("Contents")):
        last_modified_str = _text(content, "LastModified")
        last_modified = datetime.fromisoformat(
            last_modified_str.replace("Z", "+00:00")
        )
        objects.append({
            "key": _text(content, "Key"),
            "etag": _text(content, "ETag"),
            "size": int(_text(content, "Size", "0")),
            "last_modified": last_modified.isoformat(),
        })
    next_marker_el = root.find(_tag("NextMarker"))
    return {
        "name": _text(root, "Name"),
        "is_truncated": _text(root, "IsTruncated").lower() == "true",
        "next_marker": next_marker_el.text if next_marker_el is not None and next_marker_el.text else None,
        "objects": objects,
    }


def parse_initiate_multipart(xml: str) -> dict:
    root = ET.fromstring(xml)
    return {
        "bucket": _text(root, "Bucket"),
        "key": _text(root, "Key"),
        "upload_id": _text(root, "UploadId"),
    }


def parse_error_response(xml: str) -> dict:
    root = ET.fromstring(xml)
    # Error elements are not namespaced in OBS error responses
    def t(name: str) -> str:
        el = root.find(name)
        return el.text or "" if el is not None else ""
    return {
        "code": t("Code"),
        "message": t("Message"),
        "request_id": t("RequestId"),
        "host_id": t("HostId"),
    }


def serialize_complete_multipart(parts: list[tuple[int, str]]) -> str:
    root = ET.Element("CompleteMultipartUpload")
    for part_number, etag in parts:
        part_el = ET.SubElement(root, "Part")
        pn_el = ET.SubElement(part_el, "PartNumber")
        pn_el.text = str(part_number)
        etag_el = ET.SubElement(part_el, "ETag")
        etag_el.text = etag
    return ET.tostring(root, encoding="unicode")
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_xml.py -v
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/_xml.py python/tests/test_xml.py test-vectors/xml/
git commit -m "feat: XML codec + test vectors"
```

---

## Task 5: Auth — SignerV4

**Files:**
- Create: `python/src/ya_obs/_signer_v4.py`
- Create: `python/tests/test_signer_v4.py`
- Create: `test-vectors/auth/v4_header_basic.json`
- Create: `test-vectors/auth/v4_header_with_query.json`
- Create: `test-vectors/auth/v4_presign_basic.json`

- [ ] **Step 1: Write V4 auth test vectors**

`test-vectors/auth/v4_header_basic.json`:

```json
{
  "version": 1,
  "name": "v4_header_basic",
  "description": "V4 header signing for GET request, no query params",
  "input": {
    "method": "GET",
    "url": "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
    "headers": {
      "Host": "my-bucket.obs.cn-north-4.myhuaweicloud.com",
      "x-amz-date": "20240115T103000Z",
      "x-amz-content-sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    "params": {},
    "body_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "access_key": "AKIAIOSFODNN7EXAMPLE",
    "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    "region": "cn-north-4",
    "service": "s3",
    "date": "20240115"
  },
  "expected": {
    "canonical_request": "GET\n/photos/cat.jpg\n\nhost:my-bucket.obs.cn-north-4.myhuaweicloud.com\nx-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\nx-amz-date:20240115T103000Z\n\nhost;x-amz-content-sha256;x-amz-date\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "string_to_sign": "AWS4-HMAC-SHA256\n20240115T103000Z\n20240115/cn-north-4/s3/aws4_request\n",
    "authorization_prefix": "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20240115/cn-north-4/s3/aws4_request"
  }
}
```

`test-vectors/auth/v4_presign_basic.json`:

```json
{
  "version": 1,
  "name": "v4_presign_basic",
  "description": "V4 presigned URL for GET request, 1 hour expiry",
  "input": {
    "method": "GET",
    "url": "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
    "access_key": "AKIAIOSFODNN7EXAMPLE",
    "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    "region": "cn-north-4",
    "service": "s3",
    "datetime": "20240115T103000Z",
    "date": "20240115",
    "expires": 3600
  },
  "expected": {
    "required_params": [
      "X-Amz-Algorithm",
      "X-Amz-Credential",
      "X-Amz-Date",
      "X-Amz-Expires",
      "X-Amz-SignedHeaders",
      "X-Amz-Signature"
    ],
    "algorithm": "AWS4-HMAC-SHA256",
    "expires": "3600"
  }
}
```

- [ ] **Step 2: Write failing tests**

`python/tests/test_signer_v4.py`:

```python
import json
import pathlib
import hashlib
import hmac
from datetime import datetime, timezone
from ya_obs._signer_v4 import SignerV4, canonical_request, string_to_sign, signing_key
from ya_obs._models import Request

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_canonical_request_structure():
    v = load("v4_header_basic")
    i = v["input"]
    cr = canonical_request(
        method=i["method"],
        path="/photos/cat.jpg",
        query_string="",
        headers=i["headers"],
        signed_headers="host;x-amz-content-sha256;x-amz-date",
        body_sha256=i["body_sha256"],
    )
    assert cr == v["expected"]["canonical_request"]

def test_string_to_sign_structure():
    v = load("v4_header_basic")
    i = v["input"]
    cr = v["expected"]["canonical_request"]
    # string_to_sign includes a hash of canonical_request; verify prefix
    sts = string_to_sign(
        datetime_str=i["headers"]["x-amz-date"],
        date_str=i["date"],
        region=i["region"],
        service=i["service"],
        canonical_request=cr,
    )
    assert sts.startswith(v["expected"]["string_to_sign"])

def test_sign_request_adds_authorization():
    signer = SignerV4(
        access_key="AKIAIOSFODNN7EXAMPLE",
        secret_key="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        region="cn-north-4",
    )
    req = Request(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
        headers={},
    )
    signed = signer.sign(req)
    assert "Authorization" in signed.headers
    assert signed.headers["Authorization"].startswith("AWS4-HMAC-SHA256 ")

def test_presign_url_contains_required_params():
    v = load("v4_presign_basic")
    signer = SignerV4(
        access_key=v["input"]["access_key"],
        secret_key=v["input"]["secret_key"],
        region=v["input"]["region"],
    )
    url = signer.presign(
        method="GET",
        url=v["input"]["url"],
        expires=v["input"]["expires"],
    )
    for param in v["expected"]["required_params"]:
        assert param in url, f"Missing {param} in presigned URL"
    assert "AWS4-HMAC-SHA256" in url
    assert "3600" in url
```

- [ ] **Step 3: Run to verify failure**

```bash
cd python && python -m pytest tests/test_signer_v4.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 4: Write _signer_v4.py**

`python/src/ya_obs/_signer_v4.py`:

```python
from __future__ import annotations
import hashlib
import hmac
import re
from datetime import datetime, timezone
from urllib.parse import urlparse, urlencode, quote, parse_qs, urljoin

from ._models import Request


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _hmac_sha256(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode("utf-8"), hashlib.sha256).digest()


def signing_key(secret_key: str, date_str: str, region: str, service: str) -> bytes:
    k_date = _hmac_sha256(f"AWS4{secret_key}".encode("utf-8"), date_str)
    k_region = _hmac_sha256(k_date, region)
    k_service = _hmac_sha256(k_region, service)
    k_signing = _hmac_sha256(k_service, "aws4_request")
    return k_signing


def canonical_request(
    method: str,
    path: str,
    query_string: str,
    headers: dict[str, str],
    signed_headers: str,
    body_sha256: str,
) -> str:
    sorted_headers = "\n".join(
        f"{k.lower()}:{v.strip()}"
        for k, v in sorted(headers.items(), key=lambda x: x[0].lower())
        if k.lower() in signed_headers.split(";")
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

        signed_header_names = sorted(k.lower() for k in headers)
        signed_headers_str = ";".join(signed_header_names)

        query_string = parsed.query or ""

        cr = canonical_request(
            method=request.method,
            path=path,
            query_string=query_string,
            headers=headers,
            signed_headers=signed_headers_str,
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
            f"SignedHeaders={signed_headers_str}, Signature={signature}"
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
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_signer_v4.py -v
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/_signer_v4.py python/tests/test_signer_v4.py test-vectors/auth/v4_header_basic.json test-vectors/auth/v4_presign_basic.json
git commit -m "feat: SignerV4 with header and presigned URL modes"
```

---

## Task 6: Auth — SignerV2

**Files:**
- Create: `python/src/ya_obs/_signer_v2.py`
- Create: `python/tests/test_signer_v2.py`
- Create: `test-vectors/auth/v2_header_basic.json`
- Create: `test-vectors/auth/v2_header_content_md5.json`
- Create: `test-vectors/auth/v2_presign_basic.json`

- [ ] **Step 1: Write V2 test vectors**

`test-vectors/auth/v2_header_basic.json`:

```json
{
  "version": 1,
  "name": "v2_header_basic",
  "description": "OBS V2 header signing for GET request, no Content-MD5 or Content-Type",
  "input": {
    "method": "GET",
    "bucket": "my-bucket",
    "key": "photos/cat.jpg",
    "content_md5": "",
    "content_type": "",
    "date": "Tue, 15 Jan 2024 10:30:00 GMT",
    "obs_headers": {},
    "access_key": "AKIAIOSFODNN7EXAMPLE",
    "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  },
  "expected": {
    "string_to_sign": "GET\n\n\nTue, 15 Jan 2024 10:30:00 GMT\n/my-bucket/photos/cat.jpg",
    "authorization_prefix": "OBS AKIAIOSFODNN7EXAMPLE:"
  }
}
```

`test-vectors/auth/v2_header_content_md5.json`:

```json
{
  "version": 1,
  "name": "v2_header_content_md5",
  "description": "OBS V2 header signing for PUT with Content-MD5 and Content-Type",
  "input": {
    "method": "PUT",
    "bucket": "my-bucket",
    "key": "photos/cat.jpg",
    "content_md5": "rL0Y20zC+Fzt72VPzMSk2A==",
    "content_type": "image/jpeg",
    "date": "Tue, 15 Jan 2024 10:30:00 GMT",
    "obs_headers": {
      "x-obs-storage-class": "STANDARD"
    },
    "access_key": "AKIAIOSFODNN7EXAMPLE",
    "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  },
  "expected": {
    "string_to_sign": "PUT\nrL0Y20zC+Fzt72VPzMSk2A==\nimage/jpeg\nTue, 15 Jan 2024 10:30:00 GMT\nx-obs-storage-class:STANDARD\n/my-bucket/photos/cat.jpg",
    "authorization_prefix": "OBS AKIAIOSFODNN7EXAMPLE:"
  }
}
```

`test-vectors/auth/v2_presign_basic.json`:

```json
{
  "version": 1,
  "name": "v2_presign_basic",
  "description": "OBS V2 presigned URL for GET, expires at Unix timestamp 1705316200",
  "input": {
    "method": "GET",
    "bucket": "my-bucket",
    "key": "photos/cat.jpg",
    "expires_unix": 1705316200,
    "obs_headers": {},
    "access_key": "AKIAIOSFODNN7EXAMPLE",
    "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  },
  "expected": {
    "string_to_sign": "GET\n\n\n1705316200\n/my-bucket/photos/cat.jpg",
    "required_params": ["AccessKeyId", "Expires", "Signature"]
  }
}
```

- [ ] **Step 2: Write failing tests**

`python/tests/test_signer_v2.py`:

```python
import json
import pathlib
import base64
import hmac
import hashlib
from ya_obs._signer_v2 import SignerV2, build_string_to_sign, build_canonicalized_resource
from ya_obs._models import Request

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_string_to_sign_basic():
    v = load("v2_header_basic")
    i = v["input"]
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=f"/{i['bucket']}/{i['key']}",
    )
    assert sts == v["expected"]["string_to_sign"]

def test_string_to_sign_with_content_md5():
    v = load("v2_header_content_md5")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]

def test_sign_request_adds_authorization():
    signer = SignerV2(
        access_key="AKIAIOSFODNN7EXAMPLE",
        secret_key="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    )
    req = Request(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
        headers={"Date": "Tue, 15 Jan 2024 10:30:00 GMT"},
    )
    signed = signer.sign(req)
    assert "Authorization" in signed.headers
    assert signed.headers["Authorization"].startswith("OBS AKIAIOSFODNN7EXAMPLE:")

def test_presign_string_to_sign():
    v = load("v2_presign_basic")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5="",
        content_type="",
        date=str(i["expires_unix"]),
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]

def test_presign_url_contains_required_params():
    v = load("v2_presign_basic")
    i = v["input"]
    signer = SignerV2(access_key=i["access_key"], secret_key=i["secret_key"])
    url = signer.presign(
        method=i["method"],
        url=f"https://obs.cn-north-4.myhuaweicloud.com/{i['bucket']}/{i['key']}",
        expires_unix=i["expires_unix"],
        bucket=i["bucket"],
        key=i["key"],
    )
    for param in v["expected"]["required_params"]:
        assert param in url
```

- [ ] **Step 3: Run to verify failure**

```bash
cd python && python -m pytest tests/test_signer_v2.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 4: Write _signer_v2.py**

`python/src/ya_obs/_signer_v2.py`:

```python
from __future__ import annotations
import base64
import hashlib
import hmac
from datetime import datetime, timezone
from urllib.parse import urlparse, urlencode, quote

from ._models import Request

# Sub-resources that must be included in CanonicalizedResource
_OBS_SUB_RESOURCES = frozenset([
    "acl", "cors", "delete", "lifecycle", "location", "logging",
    "notification", "partNumber", "policy", "quota", "replication",
    "requestPayment", "restore", "storageClass", "storageinfo",
    "tagging", "torrent", "uploadId", "uploads", "versioning",
    "versionId", "website",
])


def build_canonicalized_resource(bucket: str, key: str, sub_resources: dict[str, str] | None = None) -> str:
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
        # Extract bucket and key from URL
        path_parts = parsed.path.lstrip("/").split("/", 1)
        # Try to infer bucket from subdomain (virtual-hosted) or path
        host = parsed.netloc
        if host.count(".") >= 3:
            # virtual-hosted: bucket.obs.region.myhuaweicloud.com
            bucket = host.split(".")[0]
            key = parsed.path.lstrip("/")
        else:
            bucket = path_parts[0] if path_parts else ""
            key = path_parts[1] if len(path_parts) > 1 else ""

        headers = dict(request.headers)
        if "Date" not in headers:
            headers["Date"] = datetime.now(timezone.utc).strftime(
                "%a, %d %b %Y %H:%M:%S GMT"
            )

        obs_headers = {k: v for k, v in headers.items() if k.lower().startswith("x-obs-")}
        resource = build_canonicalized_resource(bucket=bucket, key=key)
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
        return Request(
            method=request.method,
            url=request.url,
            headers=headers,
            params=request.params,
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
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_signer_v2.py -v
```

Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/_signer_v2.py python/tests/test_signer_v2.py test-vectors/auth/
git commit -m "feat: SignerV2 (OBS custom HMAC-SHA1) with header and presigned URL modes"
```

---

## Task 7: Retry policy

**Files:**
- Create: `python/src/ya_obs/_retry.py`
- Create: `python/tests/test_retry.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_retry.py`:

```python
import pytest
import time
from unittest.mock import MagicMock, patch
from ya_obs._retry import RetryLoop, is_retryable_status, compute_delay
from ya_obs._models import RetryPolicy

def test_is_retryable_status():
    assert is_retryable_status(500) is True
    assert is_retryable_status(503) is True
    assert is_retryable_status(408) is True
    assert is_retryable_status(429) is True
    assert is_retryable_status(404) is False
    assert is_retryable_status(403) is False
    assert is_retryable_status(200) is False

def test_compute_delay_without_jitter():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=False)
    assert compute_delay(attempt=1, policy=policy, retry_after=None) == 1.0
    assert compute_delay(attempt=2, policy=policy, retry_after=None) == 2.0
    assert compute_delay(attempt=3, policy=policy, retry_after=None) == 4.0

def test_compute_delay_caps_at_max():
    policy = RetryPolicy(base_delay=10.0, max_delay=15.0, jitter=False)
    assert compute_delay(attempt=3, policy=policy, retry_after=None) == 15.0

def test_compute_delay_respects_retry_after():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=False)
    assert compute_delay(attempt=1, policy=policy, retry_after=5.0) == 5.0

def test_compute_delay_with_jitter():
    policy = RetryPolicy(base_delay=1.0, max_delay=30.0, jitter=True)
    delays = {compute_delay(attempt=1, policy=policy, retry_after=None) for _ in range(20)}
    assert len(delays) > 1  # jitter introduces randomness
    assert all(0 <= d <= 1.0 for d in delays)

def test_retry_loop_succeeds_on_first_try():
    policy = RetryPolicy(max_attempts=3)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        return "ok"
    result = loop.run(operation)
    assert result == "ok"
    assert len(calls) == 1

def test_retry_loop_retries_on_retryable_error():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=3, base_delay=0.01, jitter=False)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        if len(calls) < 3:
            raise ServerError(code="InternalError", message="oops", status=500)
        return "ok"
    with patch("ya_obs._retry.time.sleep"):
        result = loop.run(operation)
    assert result == "ok"
    assert len(calls) == 3

def test_retry_loop_raises_after_max_attempts():
    from ya_obs._errors import ServerError
    policy = RetryPolicy(max_attempts=2, base_delay=0.01, jitter=False)
    loop = RetryLoop(policy=policy)
    def operation():
        raise ServerError(code="InternalError", message="oops", status=500)
    with patch("ya_obs._retry.time.sleep"):
        with pytest.raises(ServerError):
            loop.run(operation)

def test_retry_loop_does_not_retry_client_errors():
    from ya_obs._errors import NoSuchKey
    policy = RetryPolicy(max_attempts=3)
    loop = RetryLoop(policy=policy)
    calls = []
    def operation():
        calls.append(1)
        raise NoSuchKey(code="NoSuchKey", message="not found", status=404)
    with pytest.raises(NoSuchKey):
        loop.run(operation)
    assert len(calls) == 1
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_retry.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write _retry.py**

`python/src/ya_obs/_retry.py`:

```python
from __future__ import annotations
import logging
import random
import time
from typing import Callable, TypeVar

from ._errors import ObsError, ClientError
from ._models import RetryPolicy, RetryEvent

logger = logging.getLogger("ya_obs")
T = TypeVar("T")

_RETRYABLE_STATUSES = frozenset([408, 429, 500, 502, 503, 504])


def is_retryable_status(status: int) -> bool:
    return status in _RETRYABLE_STATUSES


def compute_delay(attempt: int, policy: RetryPolicy, retry_after: float | None) -> float:
    if retry_after is not None:
        return retry_after
    base = policy.base_delay * (2 ** (attempt - 1))
    capped = min(base, policy.max_delay)
    if policy.jitter:
        return random.uniform(0, capped)
    return capped


class RetryLoop:
    def __init__(self, policy: RetryPolicy, on_retry: Callable[[RetryEvent], None] | None = None) -> None:
        self.policy = policy
        self.on_retry = on_retry

    def run(self, operation: Callable[[], T]) -> T:
        last_error: Exception | None = None
        for attempt in range(1, self.policy.max_attempts + 1):
            try:
                return operation()
            except ClientError:
                raise
            except ObsError as e:
                if not is_retryable_status(e.status):
                    raise
                last_error = e
            except (ConnectionError, TimeoutError, OSError) as e:
                last_error = e

            if attempt == self.policy.max_attempts:
                break

            retry_after = None
            if isinstance(last_error, ObsError) and hasattr(last_error, "_retry_after"):
                retry_after = last_error._retry_after  # type: ignore[attr-defined]

            delay = compute_delay(attempt=attempt, policy=self.policy, retry_after=retry_after)
            event = RetryEvent(
                attempt=attempt,
                error=last_error,
                delay=delay,
                request_id=getattr(last_error, "request_id", None),
            )
            logger.warning(
                "ya_obs: retry attempt %d after %.2fs — %s",
                attempt,
                delay,
                last_error,
            )
            if self.on_retry:
                self.on_retry(event)
            time.sleep(delay)

        raise last_error  # type: ignore[misc]
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_retry.py -v
```

Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/_retry.py python/tests/test_retry.py
git commit -m "feat: retry loop with exponential backoff and jitter"
```

---

## Task 8: StreamingBody

**Files:**
- Create: `python/src/ya_obs/_streaming.py`
- Create: `python/tests/test_streaming.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_streaming.py`:

```python
import io
import pathlib
import tempfile
import pytest
from ya_obs._streaming import StreamingBody

def _make_body(data: bytes) -> StreamingBody:
    # Simulate httpx streaming response with an iterator
    def _iter():
        chunk_size = 4
        for i in range(0, len(data), chunk_size):
            yield data[i:i + chunk_size]
    return StreamingBody(iterator=_iter())

def test_read_buffers_all():
    body = _make_body(b"hello world")
    assert body.read() == b"hello world"

def test_iter_bytes():
    body = _make_body(b"hello world")
    chunks = list(body.iter_bytes())
    assert b"".join(chunks) == b"hello world"

def test_save_to(tmp_path):
    body = _make_body(b"file contents here")
    dest = tmp_path / "output.bin"
    body.save_to(dest)
    assert dest.read_bytes() == b"file contents here"

def test_read_twice_raises():
    body = _make_body(b"data")
    body.read()
    with pytest.raises(RuntimeError, match="already consumed"):
        body.read()

def test_iter_twice_raises():
    body = _make_body(b"data")
    list(body.iter_bytes())
    with pytest.raises(RuntimeError, match="already consumed"):
        list(body.iter_bytes())
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_streaming.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write _streaming.py**

`python/src/ya_obs/_streaming.py`:

```python
from __future__ import annotations
import pathlib
from typing import Iterator


class StreamingBody:
    def __init__(self, iterator: Iterator[bytes]) -> None:
        self._iterator = iterator
        self._consumed = False

    def _check_and_mark(self) -> None:
        if self._consumed:
            raise RuntimeError("StreamingBody already consumed")
        self._consumed = True

    def iter_bytes(self) -> Iterator[bytes]:
        self._check_and_mark()
        yield from self._iterator

    def read(self) -> bytes:
        self._check_and_mark()
        return b"".join(self._iterator)

    def save_to(self, path: str | pathlib.Path) -> None:
        self._check_and_mark()
        with open(path, "wb") as f:
            for chunk in self._iterator:
                f.write(chunk)
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_streaming.py -v
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/_streaming.py python/tests/test_streaming.py
git commit -m "feat: StreamingBody wrapper"
```

---

## Task 9: Response types

**Files:**
- Create: `python/src/ya_obs/_responses.py`

- [ ] **Step 1: Write _responses.py**

No test needed for pure dataclasses — they'll be tested implicitly via client tests.

`python/src/ya_obs/_responses.py`:

```python
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import datetime
from ._streaming import StreamingBody


@dataclass
class PutObjectResponse:
    etag: str
    version_id: str | None
    request_id: str
    client_request_id: str


@dataclass
class GetObjectResponse:
    body: StreamingBody
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


@dataclass
class HeadObjectResponse:
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


@dataclass
class DeleteObjectResponse:
    request_id: str
    client_request_id: str


@dataclass
class ObjectInfo:
    key: str
    etag: str
    size: int
    last_modified: datetime


@dataclass
class ListObjectsPage:
    objects: list[ObjectInfo]
    is_truncated: bool
    next_marker: str | None
    request_id: str
    client_request_id: str


@dataclass
class CopyObjectResponse:
    etag: str
    last_modified: datetime
    request_id: str
    client_request_id: str


@dataclass
class InitiateMultipartResponse:
    upload_id: str
    bucket: str
    key: str
    request_id: str
    client_request_id: str


@dataclass
class UploadPartResponse:
    part_number: int
    etag: str
    request_id: str
    client_request_id: str


@dataclass
class CompleteMultipartResponse:
    etag: str
    location: str
    bucket: str
    key: str
    request_id: str
    client_request_id: str
```

- [ ] **Step 2: Commit**

```bash
git add python/src/ya_obs/_responses.py
git commit -m "feat: typed response dataclasses"
```

---

## Task 10: HTTP layer

**Files:**
- Create: `python/src/ya_obs/_http.py`
- Create: `python/tests/test_http.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_http.py`:

```python
import pytest
import httpx
from pytest_httpx import HTTPXMock
from ya_obs._http import HttpClient
from ya_obs._signer_v4 import SignerV4
from ya_obs._models import Request, RetryPolicy, Timeout
from ya_obs._errors import NoSuchKey, ServerError

@pytest.fixture
def signer():
    return SignerV4(
        access_key="TESTKEY",
        secret_key="TESTSECRET",
        region="cn-north-4",
    )

@pytest.fixture
def client(signer):
    return HttpClient(
        signer=signer,
        timeout=Timeout(connect=5, read=10),
        retry_policy=RetryPolicy(max_attempts=1),
    )

def test_successful_get(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key.txt",
        status_code=200,
        content=b"hello",
        headers={"x-obs-request-id": "srv-req-123", "ETag": '"abc"'},
    )
    req = Request(method="GET", url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key.txt")
    resp = client.send(req)
    assert resp.status_code == 200
    assert resp.headers["x-obs-request-id"] == "srv-req-123"

def test_404_raises_no_such_key(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/missing.txt",
        status_code=404,
        content=b'<?xml version="1.0"?><Error><Code>NoSuchKey</Code><Message>Not found</Message><RequestId>r1</RequestId><HostId>h1</HostId></Error>',
        headers={"Content-Type": "application/xml"},
    )
    req = Request(method="GET", url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/missing.txt")
    with pytest.raises(NoSuchKey) as exc_info:
        client.send(req)
    assert exc_info.value.status == 404
    assert exc_info.value.code == "NoSuchKey"

def test_request_gets_client_id_header(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://bucket.obs.cn-north-4.myhuaweicloud.com/k",
        status_code=200,
        content=b"",
    )
    req = Request(method="GET", url="https://bucket.obs.cn-north-4.myhuaweicloud.com/k")
    client.send(req)
    sent = httpx_mock.get_requests()
    assert any("x-ya-obs-client-id" in str(r.headers) for r in sent)
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_http.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write _http.py**

`python/src/ya_obs/_http.py`:

```python
from __future__ import annotations
import logging
import uuid
from typing import Callable

import httpx

from ._errors import make_error, ObsError
from ._models import Request, Timeout, RetryPolicy, RetryEvent
from ._retry import RetryLoop
from ._xml import parse_error_response

logger = logging.getLogger("ya_obs")


class RawResponse:
    def __init__(self, response: httpx.Response, client_request_id: str) -> None:
        self._response = response
        self.client_request_id = client_request_id
        self.status_code = response.status_code
        self.headers = dict(response.headers)

    def iter_bytes(self, chunk_size: int = 65536):
        yield from self._response.iter_bytes(chunk_size=chunk_size)

    def read(self) -> bytes:
        return self._response.read()


class HttpClient:
    def __init__(
        self,
        signer,
        timeout: Timeout,
        retry_policy: RetryPolicy,
        on_retry: Callable[[RetryEvent], None] | None = None,
    ) -> None:
        self._signer = signer
        self._timeout = timeout
        self._retry_policy = retry_policy
        self._on_retry = on_retry
        self._client = httpx.Client(
            timeout=httpx.Timeout(
                connect=timeout.connect,
                read=timeout.read,
                write=None,
                pool=None,
            )
        )

    def send(self, request: Request, **overrides) -> RawResponse:
        client_request_id = str(uuid.uuid4())
        retry_loop = RetryLoop(
            policy=overrides.get("retry_policy", self._retry_policy),
            on_retry=self._on_retry,
        )

        def _attempt() -> RawResponse:
            req = Request(
                method=request.method,
                url=request.url,
                headers=dict(request.headers),
                params=request.params,
                body=request.body,
            )
            req.headers["x-ya-obs-client-id"] = client_request_id
            logger.debug("ya_obs: %s %s client_id=%s", req.method, req.url, client_request_id)

            signed = self._signer.sign(req)

            httpx_req = self._client.build_request(
                method=signed.method,
                url=signed.url,
                headers=signed.headers,
                params=signed.params,
                content=signed.body,
            )
            resp = self._client.send(httpx_req, stream=True)

            request_id = resp.headers.get("x-obs-request-id", "")
            if resp.status_code >= 400:
                body = resp.read()
                try:
                    parsed = parse_error_response(body.decode("utf-8"))
                    err = make_error(
                        code=parsed["code"],
                        message=parsed["message"],
                        status=resp.status_code,
                        request_id=parsed.get("request_id", request_id),
                        host_id=parsed.get("host_id", ""),
                        client_request_id=client_request_id,
                    )
                except Exception:
                    err = make_error(
                        code="Unknown",
                        message=body.decode("utf-8", errors="replace"),
                        status=resp.status_code,
                        request_id=request_id,
                        client_request_id=client_request_id,
                    )
                raise err

            return RawResponse(resp, client_request_id=client_request_id)

        return retry_loop.run(_attempt)

    def close(self) -> None:
        self._client.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_http.py -v
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/_http.py python/tests/test_http.py
git commit -m "feat: HTTP layer with signer injection, error parsing, client request IDs"
```

---

## Task 11: Object API — Tier 1 (put, get, head, delete, list)

**Files:**
- Create: `python/src/ya_obs/client.py`
- Create: `python/tests/test_client.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_client.py`:

```python
import pytest
from datetime import datetime, timezone
from pytest_httpx import HTTPXMock
from ya_obs.client import Client
from ya_obs._errors import NoSuchKey, NoSuchBucket

@pytest.fixture
def client():
    return Client(
        access_key="TESTKEY",
        secret_key="TESTSECRET",
        region="cn-north-4",
    )

LIST_XML = """<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>my-bucket</Name><Prefix></Prefix><MaxKeys>1000</MaxKeys>
  <IsTruncated>false</IsTruncated>
  <Contents>
    <Key>file.txt</Key>
    <LastModified>2024-01-15T10:30:00.000Z</LastModified>
    <ETag>"abc123"</ETag>
    <Size>42</Size>
  </Contents>
</ListBucketResult>"""

def test_put_object_small(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-abc"', "x-obs-request-id": "req-1"},
    )
    resp = client.put_object("my-bucket", "file.txt", b"hello world")
    assert resp.etag == '"etag-abc"'
    assert resp.request_id == "req-1"

def test_put_object_with_metadata(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-xyz"', "x-obs-request-id": "req-2"},
    )
    resp = client.put_object(
        "my-bucket", "file.txt", b"data",
        content_type="text/plain",
        metadata={"author": "sean", "project": "obs"},
    )
    sent = httpx_mock.get_requests()
    assert sent[0].headers.get("content-type") == "text/plain"
    assert sent[0].headers.get("x-obs-meta-author") == "sean"

def test_get_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=b"file contents",
        headers={
            "ETag": '"abc"',
            "Content-Type": "text/plain",
            "Content-Length": "13",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-3",
        },
    )
    resp = client.get_object("my-bucket", "file.txt")
    assert resp.etag == '"abc"'
    assert resp.content_type == "text/plain"
    assert resp.body.read() == b"file contents"

def test_get_object_range(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=206,
        content=b"llo",
        headers={"ETag": '"abc"', "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT", "x-obs-request-id": "r"},
    )
    client.get_object("my-bucket", "file.txt", range=(2, 4))
    sent = httpx_mock.get_requests()
    assert sent[0].headers.get("range") == "bytes=2-4"

def test_head_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="HEAD",
        status_code=200,
        headers={
            "ETag": '"abc"',
            "Content-Type": "image/jpeg",
            "Content-Length": "1234",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-4",
            "x-obs-meta-author": "sean",
        },
    )
    resp = client.head_object("my-bucket", "photo.jpg")
    assert resp.etag == '"abc"'
    assert resp.metadata == {"author": "sean"}

def test_delete_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(method="DELETE", status_code=204,
        headers={"x-obs-request-id": "req-5"})
    resp = client.delete_object("my-bucket", "file.txt")
    assert resp.request_id == "req-5"

def test_list_objects(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=LIST_XML.encode(),
        headers={"Content-Type": "application/xml", "x-obs-request-id": "req-6"},
    )
    page = client.list_objects_page("my-bucket")
    assert len(page.objects) == 1
    assert page.objects[0].key == "file.txt"
    assert page.objects[0].size == 42
    assert page.is_truncated is False

def test_list_objects_iterator(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=LIST_XML.encode(),
        headers={"Content-Type": "application/xml", "x-obs-request-id": "req-7"},
    )
    objects = list(client.list_objects("my-bucket"))
    assert len(objects) == 1
    assert objects[0].key == "file.txt"
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_client.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write client.py — Tier 1 operations**

`python/src/ya_obs/client.py`:

```python
from __future__ import annotations
import os
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import BinaryIO, Iterator

from ._errors import ObsError
from ._http import HttpClient
from ._models import Request, Timeout, RetryPolicy, RetryEvent
from ._responses import (
    PutObjectResponse, GetObjectResponse, HeadObjectResponse,
    DeleteObjectResponse, ListObjectsPage, ObjectInfo, CopyObjectResponse,
)
from ._signer_v4 import SignerV4
from ._signer_v2 import SignerV2
from ._streaming import StreamingBody
from ._url import build_url
from ._xml import parse_list_objects, parse_error_response

_MULTIPART_THRESHOLD = 100 * 1024 * 1024  # 100 MB
_MIN_PART_SIZE = 8 * 1024 * 1024           # 8 MB


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
        on_retry: "Callable[[RetryEvent], None] | None" = None,
    ) -> None:
        ak, sk = _resolve_credentials(access_key, secret_key)
        self._endpoint = _resolve_endpoint(region, endpoint)
        self._addressing_style = addressing_style
        self._region = region or ""

        if signing_version == "v2":
            self._signer = SignerV2(access_key=ak, secret_key=sk)
        else:
            self._signer = SignerV4(
                access_key=ak, secret_key=sk, region=self._region
            )

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

        threshold = multipart_threshold or _MULTIPART_THRESHOLD
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

        req = Request(
            method="PUT",
            url=self._url(bucket, key),
            headers=headers,
            body=body_bytes,
        )
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
        src_url = self._url(src_bucket, src_key)
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
        if hasattr(self._signer, "presign"):
            if isinstance(self._signer, SignerV2):
                import time
                expires_unix = int(time.time()) + expires
                return self._signer.presign(
                    method="GET",
                    url=url,
                    expires_unix=expires_unix,
                    bucket=bucket,
                    key=key,
                )
            return self._signer.presign(method="GET", url=url, expires=expires)
        raise NotImplementedError("Current signer does not support presigned URLs")

    def close(self) -> None:
        self._http.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_client.py -v
```

Expected: 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add python/src/ya_obs/client.py python/tests/test_client.py
git commit -m "feat: Client with put/get/head/delete/list/copy + presign"
```

---

## Task 12: Multipart upload

**Files:**
- Create: `python/src/ya_obs/_multipart.py`
- Create: `python/tests/test_multipart.py`

- [ ] **Step 1: Write failing tests**

`python/tests/test_multipart.py`:

```python
import math
import pytest
from ya_obs._multipart import compute_part_size, split_into_parts

_8MB = 8 * 1024 * 1024
_100MB = 100 * 1024 * 1024

def test_small_file_uses_8mb_parts():
    assert compute_part_size(total_size=50 * 1024 * 1024) == _8MB

def test_large_file_adapts_part_size():
    # 100 GB file should use parts larger than 8 MB to stay under 10000 parts
    size = 100 * 1024 * 1024 * 1024
    part_size = compute_part_size(total_size=size)
    assert part_size > _8MB
    assert math.ceil(size / part_size) <= 9000

def test_split_into_parts_count():
    data = b"x" * (20 * 1024 * 1024)  # 20 MB
    parts = split_into_parts(data, part_size=_8MB)
    # 20 MB / 8 MB = 3 parts (8, 8, 4)
    assert len(parts) == 3
    assert len(parts[0]) == _8MB
    assert len(parts[2]) == 4 * 1024 * 1024

def test_split_into_parts_numbering():
    data = b"y" * (10 * 1024 * 1024)
    parts = split_into_parts(data, part_size=_8MB)
    assert parts[0][0] == 1  # (part_number, data) tuples
    assert parts[1][0] == 2

def test_split_into_parts_reconstructs():
    data = b"hello world " * 1000000  # ~12 MB
    parts = split_into_parts(data, part_size=_8MB)
    reconstructed = b"".join(chunk for _, chunk in parts)
    assert reconstructed == data
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_multipart.py -v
```

Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Write _multipart.py**

`python/src/ya_obs/_multipart.py`:

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
    # 1. Initiate
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

    # 2. Upload parts
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
        # Abort on any failure
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

    # 3. Complete
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
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cd python && python -m pytest tests/test_multipart.py -v
```

Expected: 5 tests pass.

- [ ] **Step 5: Run all tests**

```bash
cd python && python -m pytest -v
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/_multipart.py python/tests/test_multipart.py
git commit -m "feat: multipart upload with adaptive part sizing and concurrency"
```

---

## Task 13: Public __init__.py and conformance vector tests

**Files:**
- Modify: `python/src/ya_obs/__init__.py`
- Create: `python/tests/vectors/test_auth_vectors.py`
- Create: `python/tests/vectors/test_xml_vectors.py`
- Create: `python/tests/vectors/test_error_vectors.py`
- Create: `test-vectors/errors/no_such_key.json`
- Create: `test-vectors/errors/no_such_bucket.json`
- Create: `test-vectors/errors/access_denied.json`

- [ ] **Step 1: Write error vectors**

`test-vectors/errors/no_such_key.json`:

```json
{
  "version": 1,
  "name": "no_such_key",
  "description": "HTTP 404 with NoSuchKey OBS error code raises NoSuchKey exception",
  "input": {
    "status": 404,
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message><RequestId>req-abc</RequestId><HostId>host-xyz</HostId></Error>"
  },
  "expected": {
    "exception_class": "NoSuchKey",
    "code": "NoSuchKey",
    "message": "The specified key does not exist.",
    "status": 404
  }
}
```

`test-vectors/errors/no_such_bucket.json`:

```json
{
  "version": 1,
  "name": "no_such_bucket",
  "description": "HTTP 404 with NoSuchBucket raises NoSuchBucket exception",
  "input": {
    "status": 404,
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>NoSuchBucket</Code><Message>The specified bucket does not exist.</Message><RequestId>req-def</RequestId><HostId>host-abc</HostId></Error>"
  },
  "expected": {
    "exception_class": "NoSuchBucket",
    "code": "NoSuchBucket",
    "message": "The specified bucket does not exist.",
    "status": 404
  }
}
```

`test-vectors/errors/access_denied.json`:

```json
{
  "version": 1,
  "name": "access_denied",
  "description": "HTTP 403 with AccessDenied raises AccessDenied exception",
  "input": {
    "status": 403,
    "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>AccessDenied</Code><Message>Access Denied.</Message><RequestId>req-ghi</RequestId><HostId>host-def</HostId></Error>"
  },
  "expected": {
    "exception_class": "AccessDenied",
    "code": "AccessDenied",
    "message": "Access Denied.",
    "status": 403
  }
}
```

- [ ] **Step 2: Write conformance vector tests**

`python/tests/vectors/__init__.py` (empty):

```python
```

`python/tests/vectors/test_auth_vectors.py`:

```python
import json
import pathlib
import pytest
from ya_obs._signer_v4 import SignerV4, canonical_request, string_to_sign
from ya_obs._signer_v2 import SignerV2, build_string_to_sign, build_canonicalized_resource

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_v4_canonical_request_vector():
    v = load("v4_header_basic")
    i = v["input"]
    cr = canonical_request(
        method=i["method"],
        path="/photos/cat.jpg",
        query_string="",
        headers=i["headers"],
        signed_headers="host;x-amz-content-sha256;x-amz-date",
        body_sha256=i["body_sha256"],
    )
    assert cr == v["expected"]["canonical_request"]

def test_v2_string_to_sign_basic_vector():
    v = load("v2_header_basic")
    i = v["input"]
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=f"/{i['bucket']}/{i['key']}",
    )
    assert sts == v["expected"]["string_to_sign"]

def test_v2_string_to_sign_content_md5_vector():
    v = load("v2_header_content_md5")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]

def test_v2_presign_string_to_sign_vector():
    v = load("v2_presign_basic")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5="",
        content_type="",
        date=str(i["expires_unix"]),
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]
```

`python/tests/vectors/test_xml_vectors.py`:

```python
import json
import pathlib
from ya_obs._xml import (
    parse_list_objects,
    parse_initiate_multipart,
    parse_error_response,
    serialize_complete_multipart,
)

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "xml"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_list_objects_vector():
    v = load("list_objects_response")
    result = parse_list_objects(v["input"]["xml"])
    assert result["name"] == v["expected"]["name"]
    assert result["is_truncated"] == v["expected"]["is_truncated"]
    assert len(result["objects"]) == len(v["expected"]["objects"])
    for got, want in zip(result["objects"], v["expected"]["objects"]):
        assert got["key"] == want["key"]
        assert got["size"] == want["size"]
        assert got["etag"] == want["etag"]

def test_initiate_multipart_vector():
    v = load("initiate_multipart_response")
    assert parse_initiate_multipart(v["input"]["xml"]) == v["expected"]

def test_error_response_vector():
    v = load("error_response")
    assert parse_error_response(v["input"]["xml"]) == v["expected"]

def test_complete_multipart_vector():
    v = load("complete_multipart_request")
    parts = [(p["part_number"], p["etag"]) for p in v["input"]["parts"]]
    assert serialize_complete_multipart(parts) == v["expected"]["xml"]
```

`python/tests/vectors/test_error_vectors.py`:

```python
import json
import pathlib
import pytest
from ya_obs._errors import make_error, NoSuchKey, NoSuchBucket, AccessDenied

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "errors"
CLASS_MAP = {"NoSuchKey": NoSuchKey, "NoSuchBucket": NoSuchBucket, "AccessDenied": AccessDenied}

@pytest.mark.parametrize("name", ["no_such_key", "no_such_bucket", "access_denied"])
def test_error_vector(name):
    v = json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))
    from ya_obs._xml import parse_error_response
    parsed = parse_error_response(v["input"]["xml"])
    err = make_error(
        code=parsed["code"],
        message=parsed["message"],
        status=v["input"]["status"],
        request_id=parsed["request_id"],
        host_id=parsed["host_id"],
    )
    expected_class = CLASS_MAP[v["expected"]["exception_class"]]
    assert isinstance(err, expected_class)
    assert err.code == v["expected"]["code"]
    assert err.status == v["expected"]["status"]
```

- [ ] **Step 3: Update __init__.py**

`python/src/ya_obs/__init__.py`:

```python
__version__ = "0.1.0"

from .client import Client
from ._errors import (
    ObsError,
    ClientError,
    ServerError,
    NoSuchKey,
    NoSuchBucket,
    AccessDenied,
)
from ._models import Timeout, RetryPolicy
from ._responses import (
    PutObjectResponse,
    GetObjectResponse,
    HeadObjectResponse,
    DeleteObjectResponse,
    ListObjectsPage,
    ObjectInfo,
    CopyObjectResponse,
)

__all__ = [
    "Client",
    "ObsError", "ClientError", "ServerError", "NoSuchKey", "NoSuchBucket", "AccessDenied",
    "Timeout", "RetryPolicy",
    "PutObjectResponse", "GetObjectResponse", "HeadObjectResponse",
    "DeleteObjectResponse", "ListObjectsPage", "ObjectInfo", "CopyObjectResponse",
]
```

- [ ] **Step 4: Run conformance tests**

```bash
cd python && python -m pytest tests/vectors/ -v
```

Expected: all conformance vector tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cd python && python -m pytest -v
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add python/src/ya_obs/__init__.py python/tests/vectors/ test-vectors/errors/
git commit -m "feat: public API surface + conformance vector tests for auth, xml, and errors"
```

---

## Task 14: AsyncClient

**Files:**
- Modify: `python/src/ya_obs/_http.py`
- Modify: `python/src/ya_obs/client.py`
- Modify: `python/src/ya_obs/__init__.py`
- Create: `python/tests/test_async_client.py`

- [ ] **Step 1: Write failing async tests**

`python/tests/test_async_client.py`:

```python
import pytest
from pytest_httpx import HTTPXMock
from ya_obs.client import AsyncClient

@pytest.fixture
def async_client():
    return AsyncClient(
        access_key="TESTKEY",
        secret_key="TESTSECRET",
        region="cn-north-4",
    )

@pytest.mark.asyncio
async def test_async_put_object(httpx_mock: HTTPXMock, async_client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-async"', "x-obs-request-id": "req-async-1"},
    )
    resp = await async_client.put_object("my-bucket", "async-file.txt", b"async content")
    assert resp.etag == '"etag-async"'
    assert resp.request_id == "req-async-1"
    await async_client.close()

@pytest.mark.asyncio
async def test_async_get_object(httpx_mock: HTTPXMock, async_client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=b"async file contents",
        headers={
            "ETag": '"def"',
            "Content-Type": "text/plain",
            "Content-Length": "19",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-async-2",
        },
    )
    resp = await async_client.get_object("my-bucket", "async-file.txt")
    data = await resp.body.read()
    assert data == b"async file contents"
    await async_client.close()

@pytest.mark.asyncio
async def test_async_context_manager(httpx_mock: HTTPXMock):
    httpx_mock.add_response(
        method="DELETE",
        status_code=204,
        headers={"x-obs-request-id": "req-async-3"},
    )
    async with AsyncClient(access_key="K", secret_key="S", region="cn-north-4") as client:
        resp = await client.delete_object("my-bucket", "file.txt")
        assert resp.request_id == "req-async-3"
```

- [ ] **Step 2: Run to verify failure**

```bash
cd python && python -m pytest tests/test_async_client.py -v
```

Expected: `ImportError` — `AsyncClient` not defined.

- [ ] **Step 3: Add AsyncHttpClient to _http.py**

Append to `python/src/ya_obs/_http.py`:

```python
class AsyncStreamingBody:
    def __init__(self, response: httpx.Response, client_request_id: str) -> None:
        self._response = response
        self.client_request_id = client_request_id
        self._consumed = False

    def _check(self) -> None:
        if self._consumed:
            raise RuntimeError("AsyncStreamingBody already consumed")
        self._consumed = True

    async def read(self) -> bytes:
        self._check()
        return await self._response.aread()

    async def iter_bytes(self, chunk_size: int = 65536):
        self._check()
        async for chunk in self._response.aiter_bytes(chunk_size):
            yield chunk


class AsyncRawResponse:
    def __init__(self, response: httpx.Response, client_request_id: str) -> None:
        self._response = response
        self.client_request_id = client_request_id
        self.status_code = response.status_code
        self.headers = dict(response.headers)

    def stream(self) -> AsyncStreamingBody:
        return AsyncStreamingBody(self._response, self.client_request_id)

    async def read(self) -> bytes:
        return await self._response.aread()


class AsyncHttpClient:
    def __init__(
        self,
        signer,
        timeout: Timeout,
        retry_policy: RetryPolicy,
        on_retry=None,
    ) -> None:
        self._signer = signer
        self._timeout = timeout
        self._retry_policy = retry_policy
        self._on_retry = on_retry
        self._client = httpx.AsyncClient(
            timeout=httpx.Timeout(
                connect=timeout.connect,
                read=timeout.read,
                write=None,
                pool=None,
            )
        )

    async def send(self, request: Request, **overrides) -> AsyncRawResponse:
        import uuid as _uuid
        client_request_id = str(_uuid.uuid4())
        req = Request(
            method=request.method,
            url=request.url,
            headers=dict(request.headers),
            params=request.params,
            body=request.body,
        )
        req.headers["x-ya-obs-client-id"] = client_request_id
        signed = self._signer.sign(req)

        httpx_req = self._client.build_request(
            method=signed.method,
            url=signed.url,
            headers=signed.headers,
            params=signed.params,
            content=signed.body,
        )
        resp = await self._client.send(httpx_req, stream=True)
        request_id = resp.headers.get("x-obs-request-id", "")

        if resp.status_code >= 400:
            body = await resp.aread()
            try:
                parsed = parse_error_response(body.decode("utf-8"))
                err = make_error(
                    code=parsed["code"],
                    message=parsed["message"],
                    status=resp.status_code,
                    request_id=parsed.get("request_id", request_id),
                    host_id=parsed.get("host_id", ""),
                    client_request_id=client_request_id,
                )
            except Exception:
                err = make_error(
                    code="Unknown",
                    message=body.decode("utf-8", errors="replace"),
                    status=resp.status_code,
                    request_id=request_id,
                    client_request_id=client_request_id,
                )
            raise err

        return AsyncRawResponse(resp, client_request_id=client_request_id)

    async def close(self) -> None:
        await self._client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        await self.close()
```

- [ ] **Step 4: Add AsyncClient to client.py**

Append to `python/src/ya_obs/client.py`:

```python
from ._http import AsyncHttpClient, AsyncStreamingBody
from ._responses import GetObjectResponse


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

        if signing_version == "v2":
            self._signer = SignerV2(access_key=ak, secret_key=sk)
        else:
            self._signer = SignerV4(access_key=ak, secret_key=sk, region=self._region)

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
        from ._streaming import StreamingBody as _SyncBody
        headers: dict[str, str] = {}
        if range is not None:
            start, end = range
            headers["Range"] = f"bytes={start}-{end}" if end is not None else f"bytes={start}-"
        req = Request(method="GET", url=self._url(bucket, key), headers=headers)
        raw = await self._http.send(req)
        stream = raw.stream()
        h = raw.headers

        # Wrap AsyncStreamingBody in a compatible interface for GetObjectResponse
        class _AsyncBodyAdapter:
            def __init__(self, s):
                self._s = s
            async def read(self):
                return await self._s.read()
            async def iter_bytes(self, chunk_size=65536):
                async for chunk in self._s.iter_bytes(chunk_size):
                    yield chunk

        adapted = _AsyncBodyAdapter(stream)
        return GetObjectResponse(
            body=adapted,  # type: ignore[arg-type]
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
```

- [ ] **Step 5: Export AsyncClient from __init__.py**

In `python/src/ya_obs/__init__.py`, add after the `Client` import:

```python
from .client import Client, AsyncClient
```

And add `"AsyncClient"` to `__all__`.

- [ ] **Step 6: Run async tests — verify pass**

```bash
cd python && python -m pytest tests/test_async_client.py -v
```

Expected: 3 tests pass.

- [ ] **Step 7: Run full suite**

```bash
cd python && python -m pytest -v
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add python/src/ya_obs/_http.py python/src/ya_obs/client.py python/src/ya_obs/__init__.py python/tests/test_async_client.py
git commit -m "feat: AsyncClient with async put/get/delete and httpx AsyncClient"
```

---

## Task 15: Final polish and README

**Files:**
- Create: `python/CHANGELOG.md`
- Modify: `README.md`

- [ ] **Step 1: Write CHANGELOG**

`python/CHANGELOG.md`:

```markdown
# Changelog

## 0.1.0 (2026-04-18)

### Added
- `Client` with `put_object`, `get_object`, `head_object`, `delete_object`, `list_objects`, `list_objects_page`, `copy_object`, `presign_get_object`
- Automatic multipart upload for objects ≥ 100 MB with adaptive part sizing and configurable concurrency
- V4 (AWS SigV4-compatible) and V2 (OBS custom HMAC-SHA1) signing
- Streaming response body with `.iter_bytes()`, `.read()`, `.save_to()`
- User metadata support (`x-obs-meta-*`)
- Exponential backoff retry with jitter
- Typed response dataclasses
- Conformance test vectors for auth (V2/V4), XML codec, and error parsing
```

- [ ] **Step 2: Write README**

`README.md`:

```markdown
# ya-obs-sdk

A clean, minimal multi-language SDK for Huawei Cloud OBS.

| Language | Package | Status |
|----------|---------|--------|
| Python | `ya-obs` (PyPI) | v0.1.0 |
| TypeScript | `ya-obs` (npm) | planned |
| Rust | `ya-obs` (crates.io) | planned |

## Python quickstart

```bash
pip install ya-obs
```

```python
from ya_obs import Client

client = Client(
    access_key="your-ak",
    secret_key="your-sk",
    region="cn-north-4",
)

# Upload
client.put_object("my-bucket", "hello.txt", b"Hello, OBS!")

# Download
resp = client.get_object("my-bucket", "hello.txt")
print(resp.body.read())

# Presigned URL (1 hour)
url = client.presign_get_object("my-bucket", "hello.txt", expires=3600)
print(url)

# List objects
for obj in client.list_objects("my-bucket"):
    print(obj.key, obj.size)
```

## Credentials

Pass explicitly or set environment variables:

```bash
export HUAWEICLOUD_SDK_AK=your-ak
export HUAWEICLOUD_SDK_SK=your-sk
```

## Signing

Default: V4 (AWS SigV4-compatible).  
For legacy deployments: `Client(..., signing_version="v2")`.

## See also

- `docs/spec/` — language-agnostic API specification
- `test-vectors/` — conformance test fixtures
```

- [ ] **Step 3: Run full test suite one final time**

```bash
cd python && python -m pytest -v --tb=short
```

Expected: all tests pass, zero failures.

- [ ] **Step 4: Final commit**

```bash
git add README.md python/CHANGELOG.md
git commit -m "docs: README and changelog for v0.1.0"
```
