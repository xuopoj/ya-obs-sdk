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

## Async

```python
from ya_obs import AsyncClient

async with AsyncClient(access_key="...", secret_key="...", region="cn-north-4") as client:
    resp = await client.get_object("my-bucket", "file.txt")
    data = await resp.body.read()
```

## See also

- `docs/spec/` — language-agnostic API specification
- `test-vectors/` — conformance test fixtures
