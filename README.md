# ya-obs-sdk

A clean, minimal multi-language SDK for Huawei Cloud OBS.

| Language | Package | Status |
|----------|---------|--------|
| Python | `ya-obs` (PyPI) | v0.2.0 |
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

## Upload progress

Pass `on_progress=` to `put_object` to receive `ProgressEvent` callbacks:

```python
from ya_obs import Client, ProgressEvent

def on_progress(ev: ProgressEvent):
    pct = 100.0 * ev.bytes_transferred / ev.total_bytes
    print(f"{ev.bytes_transferred}/{ev.total_bytes} ({pct:.1f}%)")

with Client(region="cn-north-4") as c:
    c.put_object("bucket", "big.bin", Path("big.bin"), on_progress=on_progress)
```

Fires once for single-part PUTs (at completion) and once per completed part for multipart uploads. `bytes_transferred` is monotonic even with concurrent parts. `part_number` is `None` for single-part and the 1-based part index for multipart.

## TLS / custom CA

Private OBS clusters often use an internal CA. Pass `verify` to `Client` / `AsyncClient`:

```python
import ssl

# Trust an internal CA bundle
ctx = ssl.create_default_context(cafile="/etc/ssl/corp-ca.pem")
client = Client(endpoint="https://obs.internal.example", verify=ctx)

# Disable verification (not recommended outside trusted networks)
client = Client(endpoint="https://obs.internal.example", verify=False)
```

`verify` accepts `True` (system trust, the default), `False`, a path to a PEM file, or an `ssl.SSLContext`. Mirrors `httpx`'s own `verify` parameter.

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
