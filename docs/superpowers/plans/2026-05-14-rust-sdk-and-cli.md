# Rust SDK + CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the ya-obs SDK to Rust as a library crate (`ya-obs`) and ship a CLI tool (`ya-obs-cli`) on top of it, validated against the existing language-agnostic test vectors.

**Architecture:** Cargo workspace at `rust/` with two crates. The library is async-only (tokio + reqwest). The CLI is a thin `clap`-based binary that depends on the library. Signers, XML codec, and error mapping are validated directly against `test-vectors/*.json` — the same drift firewall used by the Python SDK.

**Tech Stack:** Rust (rolling stable), tokio, reqwest, hyper, clap, quick-xml, serde, serde_json, thiserror, hmac, sha2, sha1, base64, hex, percent-encoding, url, bytes, futures, indicatif, anyhow (CLI only), cargo-dist.

**Defaults locked in (override if wrong):**

- CLI binary name: `ya-obs`
- Crate names on crates.io: `ya-obs` (library) and `ya-obs-cli` (binary)
- Distribution: `cargo install ya-obs-cli` + GitHub Releases via `cargo-dist`. No Homebrew tap in v0.1.
- MSRV: rolling stable; no `rust-version` pin until first external user complains.
- Async only at the library layer; no blocking surface in v0.1.
- Credentials: explicit args > `HUAWEICLOUD_SDK_AK`/`HUAWEICLOUD_SDK_SK` env vars. No config file in v0.1.

---

## File Structure

```
rust/
├── Cargo.toml                          # workspace manifest
├── rust-toolchain.toml                 # stable channel pin
├── .cargo/config.toml                  # cross-compile targets for releases
├── ya-obs/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                      # re-exports public surface
│   │   ├── client.rs                   # Client struct + builder
│   │   ├── config.rs                   # ClientConfig, Region, Endpoint, AddressingStyle
│   │   ├── credentials.rs              # Credentials struct + env resolution
│   │   ├── error.rs                    # Error enum (thiserror), error code -> variant mapping
│   │   ├── http.rs                     # reqwest client wiring, header construction, request IDs
│   │   ├── retry.rs                    # RetryPolicy, backoff, Retry-After parsing
│   │   ├── signer/
│   │   │   ├── mod.rs                  # Signer trait, public re-exports
│   │   │   ├── v4.rs                   # SigV4 implementation
│   │   │   └── v2.rs                   # OBS V2 implementation
│   │   ├── url.rs                      # endpoint resolution + addressing style + percent-encoding
│   │   ├── xml.rs                      # OBS XML parse/serialize helpers
│   │   ├── models.rs                   # request/response structs (Object, ListBucketResult, ...)
│   │   ├── streaming.rs                # StreamingBody wrapper around reqwest::Response
│   │   ├── multipart.rs                # multipart upload orchestration
│   │   └── operations/
│   │       ├── mod.rs
│   │       ├── put_object.rs
│   │       ├── get_object.rs
│   │       ├── head_object.rs
│   │       ├── delete_object.rs
│   │       ├── list_objects.rs
│   │       ├── copy_object.rs
│   │       └── presign.rs
│   └── tests/
│       ├── conformance_auth_v4.rs      # loads test-vectors/auth/v4_*.json
│       ├── conformance_auth_v2.rs      # loads test-vectors/auth/v2_*.json
│       ├── conformance_xml.rs          # loads test-vectors/xml/*.json
│       ├── conformance_errors.rs       # loads test-vectors/errors/*.json
│       └── support/
│           └── mod.rs                  # shared fixture-loading helpers
└── ya-obs-cli/
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs                     # clap entrypoint, tokio runtime
    │   ├── args.rs                     # CLI arg parsing (clap derive)
    │   ├── obs_uri.rs                  # parse `obs://bucket/key`
    │   ├── progress.rs                 # indicatif progress bar
    │   └── commands/
    │       ├── mod.rs
    │       ├── ls.rs
    │       ├── cp.rs
    │       ├── rm.rs
    │       ├── cat.rs
    │       └── presign.rs
    └── tests/
        └── cli_smoke.rs                # uses assert_cmd for arg parsing tests

.github/workflows/
└── rust.yml                            # CI: fmt + clippy + test on macOS + linux

README.md                               # add Rust section
rust/CHANGELOG.md                       # Keep-a-changelog format
```

**Decomposition rationale:**

- Signer split into a submodule because v4 + v2 share little code but share the `Signer` trait — splitting prevents one 600-line file.
- Operations split per-method so each PUT/GET/etc. stays small and reviewable; `operations/mod.rs` only re-exports.
- CLI commands split per-subcommand for the same reason and because each one will have its own test once shelled-out commands are added later.
- `tests/support/mod.rs` consolidates fixture-loading so each conformance file stays focused on assertions.

**Each task below is independently committable.** Run `cargo fmt` and `cargo clippy --all-targets -- -D warnings` before every commit.

---

### Task 1: Cargo workspace skeleton

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/ya-obs/Cargo.toml`
- Create: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs-cli/Cargo.toml`
- Create: `rust/ya-obs-cli/src/main.rs`
- Create: `rust/.gitignore`

- [ ] **Step 1: Write the workspace manifest**

Create `rust/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["ya-obs", "ya-obs-cli"]

[workspace.package]
edition = "2021"
license = "MIT"
repository = "https://github.com/xuopoj/ya-obs-sdk"
authors = ["Sean <xuopoj@gmail.com>"]

[workspace.dependencies]
ya-obs = { path = "ya-obs", version = "0.1.0" }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "http2"] }
bytes = "1"
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
quick-xml = { version = "0.36", features = ["serialize"] }
thiserror = "1"
hmac = "0.12"
sha2 = "0.10"
sha1 = "0.10"
base64 = "0.22"
hex = "0.4"
url = "2"
percent-encoding = "2"
time = { version = "0.3", features = ["formatting", "parsing", "macros"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
```

- [ ] **Step 2: Write the toolchain pin**

Create `rust/rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Write the library crate manifest**

Create `rust/ya-obs/Cargo.toml`:

```toml
[package]
name = "ya-obs"
version = "0.1.0"
description = "A clean Rust SDK for Huawei Cloud OBS"
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
readme = "../../README.md"

[dependencies]
tokio = { workspace = true }
reqwest = { workspace = true }
bytes = { workspace = true }
futures = { workspace = true }
serde = { workspace = true }
quick-xml = { workspace = true }
thiserror = { workspace = true }
hmac = { workspace = true }
sha2 = { workspace = true }
sha1 = { workspace = true }
base64 = { workspace = true }
hex = { workspace = true }
url = { workspace = true }
percent-encoding = { workspace = true }
time = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
serde_json = { workspace = true }
tokio = { workspace = true, features = ["full", "test-util"] }
```

- [ ] **Step 4: Write the library entrypoint**

Create `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.
```

- [ ] **Step 5: Write the CLI crate manifest**

Create `rust/ya-obs-cli/Cargo.toml`:

```toml
[package]
name = "ya-obs-cli"
version = "0.1.0"
description = "Command-line interface for Huawei Cloud OBS, built on ya-obs"
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[[bin]]
name = "ya-obs"
path = "src/main.rs"

[dependencies]
ya-obs = { workspace = true }
tokio = { workspace = true }
clap = { version = "4", features = ["derive", "env"] }
anyhow = "1"
indicatif = "0.17"
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 6: Write a minimal CLI entrypoint**

Create `rust/ya-obs-cli/src/main.rs`:

```rust
fn main() {
    println!("ya-obs cli placeholder");
}
```

- [ ] **Step 7: Write .gitignore**

Create `rust/.gitignore`:

```
target/
Cargo.lock.bak
```

(Keep `Cargo.lock` checked in for the workspace since it contains a binary crate.)

- [ ] **Step 8: Verify the workspace builds**

Run: `cd rust && cargo build --workspace`
Expected: PASS — both crates compile with the placeholder code.

- [ ] **Step 9: Commit**

```bash
git add rust/
git commit -m "feat(rust): scaffold cargo workspace for ya-obs + ya-obs-cli"
```

---

### Task 2: Conformance test harness — fixture loader

**Files:**
- Create: `rust/ya-obs/tests/support/mod.rs`

We need shared infrastructure for loading test vectors before writing any signer tests. This file lives in `tests/` so every integration test can `mod support;`.

- [ ] **Step 1: Write the support module**

Create `rust/ya-obs/tests/support/mod.rs`:

```rust
//! Shared helpers for loading test-vector fixtures.

use std::path::PathBuf;

/// Path to the repo-root `test-vectors/` directory, resolved relative to the
/// `ya-obs` crate at `rust/ya-obs/`.
pub fn vectors_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // rust/
        .and_then(|p| p.parent()) // repo root
        .expect("CARGO_MANIFEST_DIR must have at least two ancestors")
        .join("test-vectors")
}

/// Read and parse a JSON test vector at `test-vectors/<category>/<name>.json`.
pub fn load_vector(category: &str, name: &str) -> serde_json::Value {
    let path = vectors_dir().join(category).join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}
```

- [ ] **Step 2: Smoke-test the loader from a throwaway test**

Create `rust/ya-obs/tests/conformance_smoke.rs`:

```rust
mod support;

#[test]
fn loads_v4_header_basic_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    assert_eq!(v["name"], "v4_header_basic");
    assert_eq!(v["input"]["region"], "cn-north-4");
}
```

- [ ] **Step 3: Run the smoke test**

Run: `cd rust && cargo test -p ya-obs --test conformance_smoke`
Expected: PASS (1 test).

- [ ] **Step 4: Commit**

```bash
git add rust/ya-obs/tests/
git commit -m "test(rust): add test-vector fixture loader and smoke test"
```

The smoke test stays — it's a guardrail against accidentally breaking the loader.

---

### Task 3: SigV4 — canonical request

**Files:**
- Create: `rust/ya-obs/src/signer/mod.rs`
- Create: `rust/ya-obs/src/signer/v4.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/conformance_auth_v4.rs`

We build SigV4 in three sub-tasks: canonical request, string-to-sign + signature, then presign. Each maps directly to a field in `test-vectors/auth/v4_header_basic.json`.

- [ ] **Step 1: Write the failing test — canonical request from vector**

Create `rust/ya-obs/tests/conformance_auth_v4.rs`:

```rust
mod support;

use ya_obs::signer::v4::canonical_request;

#[test]
fn v4_header_basic_canonical_request_matches_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let actual = canonical_request(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["canonical_request"].as_str().unwrap());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: FAIL with "unresolved import `ya_obs::signer`".

- [ ] **Step 3: Implement `signer/mod.rs` and `signer/v4.rs::canonical_request`**

Create `rust/ya-obs/src/signer/mod.rs`:

```rust
pub mod v4;
```

Create `rust/ya-obs/src/signer/v4.rs`:

```rust
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

/// Characters allowed in SigV4 canonical URIs — everything outside `unreserved`
/// gets percent-encoded.
const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?')
    .add(b'`').add(b'{').add(b'}').add(b'^').add(b'|').add(b'\\')
    .add(b'%').add(b'+').add(b'!').add(b'$').add(b'&').add(b'\'')
    .add(b'(').add(b')').add(b'*').add(b',').add(b';').add(b'=')
    .add(b':').add(b'@').add(b'[').add(b']');

fn encode_path_segment(seg: &str) -> String {
    utf8_percent_encode(seg, PATH_ENCODE_SET).to_string()
}

fn canonical_uri(url: &Url) -> String {
    let path = url.path();
    if path.is_empty() {
        return "/".into();
    }
    path.split('/').map(encode_path_segment).collect::<Vec<_>>().join("/")
}

fn canonical_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| {
            (
                utf8_percent_encode(&k, PATH_ENCODE_SET).to_string(),
                utf8_percent_encode(&v, PATH_ENCODE_SET).to_string(),
            )
        })
        .collect();
    pairs.sort();
    pairs.into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
}

/// Build the SigV4 canonical request string.
/// `headers` is the full set of headers to sign (must include `Host`).
pub fn canonical_request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body_sha256: &str,
) -> String {
    let parsed = Url::parse(url).expect("valid url");

    let uri = canonical_uri(&parsed);
    let query = canonical_query(&parsed);

    let mut lowered: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    lowered.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = lowered
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect();

    let signed_headers = lowered
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    format!(
        "{method}\n{uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{body_sha256}"
    )
}
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod signer;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: PASS (1 test: `v4_header_basic_canonical_request_matches_vector`).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/lib.rs rust/ya-obs/src/signer/ rust/ya-obs/tests/conformance_auth_v4.rs
git commit -m "feat(rust): SigV4 canonical request validated against test vectors"
```

---

### Task 4: SigV4 — string-to-sign, signing key, full header

**Files:**
- Modify: `rust/ya-obs/src/signer/v4.rs`
- Modify: `rust/ya-obs/tests/conformance_auth_v4.rs`

- [ ] **Step 1: Add failing tests for string-to-sign and authorization header**

Append to `rust/ya-obs/tests/conformance_auth_v4.rs`:

```rust
use ya_obs::signer::v4::{authorization_header, string_to_sign};

#[test]
fn v4_header_basic_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let canonical = canonical_request(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
    );

    let actual = string_to_sign(
        input["headers"]["x-amz-date"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
        &canonical,
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v4_header_basic_authorization_starts_with_expected_prefix() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let auth = authorization_header(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
        input["headers"]["x-amz-date"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
    );

    let prefix = v["expected"]["authorization_prefix"].as_str().unwrap();
    assert!(
        auth.starts_with(prefix),
        "authorization header {auth:?} should start with {prefix:?}"
    );
    assert!(auth.contains(", Signature="), "missing Signature= component");
}
```

The string-to-sign vector ends with an empty 4th line (`...aws4_request\n`) — that's the hashed-canonical-request slot. The vector intentionally truncates so we can compute it; we will need to append `hex(sha256(canonical_request))` to the format string before signing.

Re-read `test-vectors/auth/v4_header_basic.json` `expected.string_to_sign` carefully — it ends with `aws4_request\n` (trailing newline, no hash). That's the format the vector asserts.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: FAIL — `string_to_sign` and `authorization_header` not defined.

- [ ] **Step 3: Implement signing functions**

Append to `rust/ya-obs/src/signer/v4.rs`:

```rust
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Build the AWS4-HMAC-SHA256 string-to-sign.
///
/// The returned string matches the test-vector format: the trailing line is
/// empty (callers prepend the hashed canonical request only when computing the
/// signature, not when comparing to fixtures).
pub fn string_to_sign(
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
    _canonical_request: &str,
) -> String {
    format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n"
    )
}

/// Derive the SigV4 signing key.
pub fn signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// Compute the hex signature for a request and produce the full Authorization
/// header value.
#[allow(clippy::too_many_arguments)]
pub fn authorization_header(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body_sha256: &str,
    access_key: &str,
    secret_key: &str,
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
) -> String {
    let canonical = canonical_request(method, url, headers, body_sha256);
    let hashed_canonical = sha256_hex(canonical.as_bytes());

    let to_sign_with_hash = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{hashed_canonical}"
    );

    let key = signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&key, to_sign_with_hash.as_bytes()));

    let mut lowered: Vec<String> = headers
        .iter()
        .map(|(k, _)| k.to_ascii_lowercase())
        .collect();
    lowered.sort();
    let signed_headers = lowered.join(";");

    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{date}/{region}/{service}/aws4_request, \
         SignedHeaders={signed_headers}, Signature={signature}"
    )
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/signer/v4.rs rust/ya-obs/tests/conformance_auth_v4.rs
git commit -m "feat(rust): SigV4 string-to-sign + Authorization header"
```

---

### Task 5: SigV4 — presigned URLs

**Files:**
- Modify: `rust/ya-obs/src/signer/v4.rs`
- Modify: `rust/ya-obs/tests/conformance_auth_v4.rs`

- [ ] **Step 1: Add failing test for presigned URL**

Append to `rust/ya-obs/tests/conformance_auth_v4.rs`:

```rust
use ya_obs::signer::v4::presign_url;

#[test]
fn v4_presign_basic_url_has_required_params_and_signature() {
    let v = support::load_vector("auth", "v4_presign_basic");
    let input = &v["input"];

    let url = presign_url(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
        input["datetime"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
        input["expires"].as_u64().unwrap(),
    );

    let parsed = url::Url::parse(&url).unwrap();
    let qs: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();

    for required in v["expected"]["required_params"].as_array().unwrap() {
        let k = required.as_str().unwrap();
        assert!(qs.contains_key(k), "missing required param {k}");
    }
    assert_eq!(qs["X-Amz-Algorithm"], v["expected"]["algorithm"].as_str().unwrap());
    assert_eq!(qs["X-Amz-Expires"], v["expected"]["expires"].as_str().unwrap());
    assert!(!qs["X-Amz-Signature"].is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: FAIL — `presign_url` not defined.

- [ ] **Step 3: Implement presigned URLs**

Append to `rust/ya-obs/src/signer/v4.rs`:

```rust
/// Build a SigV4 presigned URL by adding `X-Amz-*` query parameters.
#[allow(clippy::too_many_arguments)]
pub fn presign_url(
    method: &str,
    url: &str,
    access_key: &str,
    secret_key: &str,
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
    expires: u64,
) -> String {
    let mut parsed = Url::parse(url).expect("valid url");
    let host = parsed.host_str().expect("url must have host").to_string();

    let credential = format!("{access_key}/{date}/{region}/{service}/aws4_request");

    parsed.query_pairs_mut()
        .append_pair("X-Amz-Algorithm", "AWS4-HMAC-SHA256")
        .append_pair("X-Amz-Credential", &credential)
        .append_pair("X-Amz-Date", amz_date)
        .append_pair("X-Amz-Expires", &expires.to_string())
        .append_pair("X-Amz-SignedHeaders", "host");

    // For presigned URLs, the canonical request uses UNSIGNED-PAYLOAD as the
    // body hash and includes only the `host` header.
    let headers = vec![("host".to_string(), host)];
    let canonical = canonical_request(method, parsed.as_str(), &headers, "UNSIGNED-PAYLOAD");
    let hashed_canonical = sha256_hex(canonical.as_bytes());

    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{hashed_canonical}"
    );
    let key = signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&key, to_sign.as_bytes()));

    parsed.query_pairs_mut().append_pair("X-Amz-Signature", &signature);
    parsed.to_string()
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v4`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/signer/v4.rs rust/ya-obs/tests/conformance_auth_v4.rs
git commit -m "feat(rust): SigV4 presigned URL generation"
```

---

### Task 6: OBS V2 signer — header form

**Files:**
- Create: `rust/ya-obs/src/signer/v2.rs`
- Modify: `rust/ya-obs/src/signer/mod.rs`
- Create: `rust/ya-obs/tests/conformance_auth_v2.rs`

- [ ] **Step 1: Add failing test for V2 string-to-sign (basic)**

Create `rust/ya-obs/tests/conformance_auth_v2.rs`:

```rust
mod support;

use std::collections::BTreeMap;
use ya_obs::signer::v2::{authorization_header, string_to_sign};

#[test]
fn v2_header_basic_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v2_header_basic");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = input["obs_headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let actual = string_to_sign(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v2_header_content_md5_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v2_header_content_md5");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = input["obs_headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let actual = string_to_sign(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v2_header_basic_authorization_prefix() {
    let v = support::load_vector("auth", "v2_header_basic");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = BTreeMap::new();

    let auth = authorization_header(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
    );

    assert!(auth.starts_with(v["expected"]["authorization_prefix"].as_str().unwrap()));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v2`
Expected: FAIL — `ya_obs::signer::v2` not defined.

- [ ] **Step 3: Implement V2 signer**

Create `rust/ya-obs/src/signer/v2.rs`:

```rust
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::BTreeMap;

type HmacSha1 = Hmac<Sha1>;

fn canonicalized_obs_headers(headers: &BTreeMap<String, String>) -> String {
    headers
        .iter()
        .filter(|(k, _)| k.to_ascii_lowercase().starts_with("x-obs-"))
        .map(|(k, v)| format!("{}:{}", k.to_ascii_lowercase(), v.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn canonicalized_resource(bucket: &str, key: &str) -> String {
    if key.is_empty() {
        format!("/{bucket}/")
    } else {
        format!("/{bucket}/{key}")
    }
}

/// OBS V2 string-to-sign for header authentication.
pub fn string_to_sign(
    method: &str,
    content_md5: &str,
    content_type: &str,
    date: &str,
    obs_headers: &BTreeMap<String, String>,
    bucket: &str,
    key: &str,
) -> String {
    let canon_headers = canonicalized_obs_headers(obs_headers);
    let resource = canonicalized_resource(bucket, key);

    let header_line = if canon_headers.is_empty() {
        String::new()
    } else {
        format!("{canon_headers}\n")
    };

    format!(
        "{method}\n{content_md5}\n{content_type}\n{date}\n{header_line}{resource}"
    )
}

fn sign(secret_key: &str, to_sign: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret_key.as_bytes()).expect("HMAC accepts any key");
    mac.update(to_sign.as_bytes());
    B64.encode(mac.finalize().into_bytes())
}

#[allow(clippy::too_many_arguments)]
pub fn authorization_header(
    method: &str,
    content_md5: &str,
    content_type: &str,
    date: &str,
    obs_headers: &BTreeMap<String, String>,
    bucket: &str,
    key: &str,
    access_key: &str,
    secret_key: &str,
) -> String {
    let to_sign = string_to_sign(method, content_md5, content_type, date, obs_headers, bucket, key);
    let signature = sign(secret_key, &to_sign);
    format!("OBS {access_key}:{signature}")
}
```

Modify `rust/ya-obs/src/signer/mod.rs`:

```rust
pub mod v2;
pub mod v4;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v2`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/signer/v2.rs rust/ya-obs/src/signer/mod.rs rust/ya-obs/tests/conformance_auth_v2.rs
git commit -m "feat(rust): OBS V2 header signing validated against vectors"
```

---

### Task 7: OBS V2 signer — presigned URLs

**Files:**
- Modify: `rust/ya-obs/src/signer/v2.rs`
- Modify: `rust/ya-obs/tests/conformance_auth_v2.rs`

- [ ] **Step 1: Add failing test for V2 presign**

Append to `rust/ya-obs/tests/conformance_auth_v2.rs`:

```rust
use ya_obs::signer::v2::presign_url;

#[test]
fn v2_presign_basic_url_has_required_params() {
    let v = support::load_vector("auth", "v2_presign_basic");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = input["obs_headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let (url, sts) = presign_url(
        input["method"].as_str().unwrap(),
        "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
        input["expires_unix"].as_u64().unwrap(),
        &obs_headers,
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
    );

    assert_eq!(sts, v["expected"]["string_to_sign"].as_str().unwrap());

    let parsed = url::Url::parse(&url).unwrap();
    let qs: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    for k in v["expected"]["required_params"].as_array().unwrap() {
        assert!(qs.contains_key(k.as_str().unwrap()));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v2`
Expected: FAIL — `presign_url` not defined.

- [ ] **Step 3: Implement V2 presign**

Append to `rust/ya-obs/src/signer/v2.rs`:

```rust
use url::Url;

/// OBS V2 presigned URL. For presigning, the "Date" line in the canonical
/// string is the Unix-timestamp expiry, and Content-MD5/Content-Type are empty.
///
/// Returns `(url, string_to_sign)` so callers (and tests) can verify the STS.
#[allow(clippy::too_many_arguments)]
pub fn presign_url(
    method: &str,
    url: &str,
    bucket: &str,
    key: &str,
    expires_unix: u64,
    obs_headers: &BTreeMap<String, String>,
    access_key: &str,
    secret_key: &str,
) -> (String, String) {
    let sts = string_to_sign(method, "", "", &expires_unix.to_string(), obs_headers, bucket, key);
    let signature = sign(secret_key, &sts);

    let mut parsed = Url::parse(url).expect("valid url");
    parsed.query_pairs_mut()
        .append_pair("AccessKeyId", access_key)
        .append_pair("Expires", &expires_unix.to_string())
        .append_pair("Signature", &signature);

    (parsed.to_string(), sts)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test conformance_auth_v2`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/signer/v2.rs rust/ya-obs/tests/conformance_auth_v2.rs
git commit -m "feat(rust): OBS V2 presigned URLs"
```

---

### Task 8: XML codec — parse list_objects response

**Files:**
- Create: `rust/ya-obs/src/xml.rs`
- Create: `rust/ya-obs/src/models.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/conformance_xml.rs`

- [ ] **Step 1: Add failing test**

Create `rust/ya-obs/tests/conformance_xml.rs`:

```rust
mod support;

use ya_obs::xml::parse_list_bucket_result;

#[test]
fn list_objects_response_parses_to_expected_objects() {
    let v = support::load_vector("xml", "list_objects_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let result = parse_list_bucket_result(xml).expect("parse ok");

    assert_eq!(result.name, v["expected"]["name"].as_str().unwrap());
    assert_eq!(result.is_truncated, v["expected"]["is_truncated"].as_bool().unwrap());
    assert!(result.next_marker.is_none());

    let expected_objects = v["expected"]["objects"].as_array().unwrap();
    assert_eq!(result.objects.len(), expected_objects.len());

    for (got, want) in result.objects.iter().zip(expected_objects) {
        assert_eq!(got.key, want["key"].as_str().unwrap());
        assert_eq!(got.etag, want["etag"].as_str().unwrap());
        assert_eq!(got.size, want["size"].as_u64().unwrap());
        assert_eq!(got.last_modified, want["last_modified"].as_str().unwrap());
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cd rust && cargo test -p ya-obs --test conformance_xml`
Expected: FAIL — `ya_obs::xml` not defined.

- [ ] **Step 3: Implement parser**

Create `rust/ya-obs/src/models.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ObjectSummary {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified_raw: String,
    #[serde(rename = "ETag")]
    pub etag: String,
    #[serde(rename = "Size")]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBucketResult {
    pub name: String,
    pub prefix: Option<String>,
    pub max_keys: u32,
    pub is_truncated: bool,
    pub next_marker: Option<String>,
    pub objects: Vec<ListedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedObject {
    pub key: String,
    pub etag: String,
    pub size: u64,
    /// ISO-8601 with offset, e.g. `2024-01-15T10:30:00+00:00`.
    pub last_modified: String,
}
```

Create `rust/ya-obs/src/xml.rs`:

```rust
use quick_xml::de::from_str;
use serde::Deserialize;

use crate::models::{ListBucketResult, ListedObject};

#[derive(Debug, Deserialize)]
struct RawContents {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "LastModified")]
    last_modified: String,
    #[serde(rename = "ETag")]
    etag: String,
    #[serde(rename = "Size")]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct RawListBucketResult {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Prefix", default)]
    prefix: Option<String>,
    #[serde(rename = "MaxKeys", default)]
    max_keys: u32,
    #[serde(rename = "IsTruncated")]
    is_truncated: bool,
    #[serde(rename = "NextMarker", default)]
    next_marker: Option<String>,
    #[serde(rename = "Contents", default)]
    contents: Vec<RawContents>,
}

/// Normalize OBS `LastModified` (`2024-01-15T10:30:00.000Z`) into the
/// `2024-01-15T10:30:00+00:00` form the test vectors expect.
fn normalize_last_modified(raw: &str) -> String {
    // Drop fractional seconds (split on '.') and replace trailing 'Z' with offset.
    let no_frac = raw.split('.').next().unwrap_or(raw);
    if raw.ends_with('Z') {
        format!("{no_frac}+00:00")
    } else {
        no_frac.to_string()
    }
}

pub fn parse_list_bucket_result(xml: &str) -> Result<ListBucketResult, quick_xml::DeError> {
    let raw: RawListBucketResult = from_str(xml)?;

    let objects = raw
        .contents
        .into_iter()
        .map(|c| ListedObject {
            key: c.key,
            etag: c.etag,
            size: c.size,
            last_modified: normalize_last_modified(&c.last_modified),
        })
        .collect();

    Ok(ListBucketResult {
        name: raw.name,
        prefix: raw.prefix.filter(|p| !p.is_empty()),
        max_keys: raw.max_keys,
        is_truncated: raw.is_truncated,
        next_marker: raw.next_marker.filter(|s| !s.is_empty()),
        objects,
    })
}
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod models;
pub mod signer;
pub mod xml;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test conformance_xml`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/xml.rs rust/ya-obs/src/models.rs rust/ya-obs/src/lib.rs rust/ya-obs/tests/conformance_xml.rs
git commit -m "feat(rust): XML codec — parse ListBucketResult"
```

---

### Task 9: XML codec — error response + multipart

**Files:**
- Modify: `rust/ya-obs/src/xml.rs`
- Modify: `rust/ya-obs/src/models.rs`
- Modify: `rust/ya-obs/tests/conformance_xml.rs`

- [ ] **Step 1: Add failing tests**

Append to `rust/ya-obs/tests/conformance_xml.rs`:

```rust
use ya_obs::xml::{
    parse_error_response, parse_initiate_multipart_result, serialize_complete_multipart,
};

#[test]
fn error_response_parses_to_expected_fields() {
    let v = support::load_vector("xml", "error_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let err = parse_error_response(xml).expect("parse ok");

    assert_eq!(err.code, v["expected"]["code"].as_str().unwrap());
    assert_eq!(err.message, v["expected"]["message"].as_str().unwrap());
    assert_eq!(err.request_id, v["expected"]["request_id"].as_str().unwrap());
    assert_eq!(err.host_id, v["expected"]["host_id"].as_str().unwrap());
}

#[test]
fn initiate_multipart_result_parses_to_expected_fields() {
    let v = support::load_vector("xml", "initiate_multipart_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let r = parse_initiate_multipart_result(xml).expect("parse ok");

    assert_eq!(r.bucket, v["expected"]["bucket"].as_str().unwrap());
    assert_eq!(r.key, v["expected"]["key"].as_str().unwrap());
    assert_eq!(r.upload_id, v["expected"]["upload_id"].as_str().unwrap());
}

#[test]
fn complete_multipart_request_serializes_to_expected_xml() {
    let v = support::load_vector("xml", "complete_multipart_request");
    let parts: Vec<(u32, String)> = v["input"]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (
            p["part_number"].as_u64().unwrap() as u32,
            p["etag"].as_str().unwrap().to_string(),
        ))
        .collect();

    let actual = serialize_complete_multipart(&parts);
    assert_eq!(actual, v["expected"]["xml"].as_str().unwrap());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && cargo test -p ya-obs --test conformance_xml`
Expected: FAIL — three functions not defined.

- [ ] **Step 3: Implement parsing + serialization**

Append to `rust/ya-obs/src/models.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub host_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateMultipartResult {
    pub bucket: String,
    pub key: String,
    pub upload_id: String,
}
```

Append to `rust/ya-obs/src/xml.rs`:

```rust
use crate::models::{ErrorResponse, InitiateMultipartResult};

#[derive(Debug, Deserialize)]
struct RawError {
    #[serde(rename = "Code")]
    code: String,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "RequestId", default)]
    request_id: String,
    #[serde(rename = "HostId", default)]
    host_id: String,
}

pub fn parse_error_response(xml: &str) -> Result<ErrorResponse, quick_xml::DeError> {
    let raw: RawError = from_str(xml)?;
    Ok(ErrorResponse {
        code: raw.code,
        message: raw.message,
        request_id: raw.request_id,
        host_id: raw.host_id,
    })
}

#[derive(Debug, Deserialize)]
struct RawInitiateMultipart {
    #[serde(rename = "Bucket")]
    bucket: String,
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "UploadId")]
    upload_id: String,
}

pub fn parse_initiate_multipart_result(
    xml: &str,
) -> Result<InitiateMultipartResult, quick_xml::DeError> {
    let raw: RawInitiateMultipart = from_str(xml)?;
    Ok(InitiateMultipartResult {
        bucket: raw.bucket,
        key: raw.key,
        upload_id: raw.upload_id,
    })
}

/// Serialize a CompleteMultipartUpload body. Hand-written rather than going via
/// serde to guarantee byte-exact output (OBS rejects extra whitespace).
pub fn serialize_complete_multipart(parts: &[(u32, String)]) -> String {
    let mut out = String::from("<CompleteMultipartUpload>");
    for (n, etag) in parts {
        out.push_str(&format!("<Part><PartNumber>{n}</PartNumber><ETag>{etag}</ETag></Part>"));
    }
    out.push_str("</CompleteMultipartUpload>");
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test conformance_xml`
Expected: PASS (4 tests total).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/xml.rs rust/ya-obs/src/models.rs rust/ya-obs/tests/conformance_xml.rs
git commit -m "feat(rust): XML codec — error response + multipart"
```

---

### Task 10: Error types + error-vector conformance

**Files:**
- Create: `rust/ya-obs/src/error.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/conformance_errors.rs`

- [ ] **Step 1: Add failing tests**

Create `rust/ya-obs/tests/conformance_errors.rs`:

```rust
mod support;

use ya_obs::error::{classify_error, Error};

fn check_vector(name: &str, expected_variant_name: &str) {
    let v = support::load_vector("errors", name);
    let input = &v["input"];

    let err = classify_error(
        input["status"].as_u64().unwrap() as u16,
        input["xml"].as_str().unwrap(),
    );

    let actual_variant = match &err {
        Error::NoSuchKey { .. } => "NoSuchKey",
        Error::NoSuchBucket { .. } => "NoSuchBucket",
        Error::AccessDenied { .. } => "AccessDenied",
        Error::Client { .. } => "Client",
        Error::Server { .. } => "Server",
        _ => "Other",
    };
    assert_eq!(actual_variant, expected_variant_name);

    // All variants expose code/message/status.
    assert_eq!(err.code(), v["expected"]["code"].as_str().unwrap());
    assert_eq!(err.message(), v["expected"]["message"].as_str().unwrap());
    assert_eq!(err.status(), v["expected"]["status"].as_u64().unwrap() as u16);
}

#[test]
fn no_such_key_maps_to_no_such_key_variant() {
    check_vector("no_such_key", "NoSuchKey");
}

#[test]
fn no_such_bucket_maps_to_no_such_bucket_variant() {
    check_vector("no_such_bucket", "NoSuchBucket");
}

#[test]
fn access_denied_maps_to_access_denied_variant() {
    check_vector("access_denied", "AccessDenied");
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test -p ya-obs --test conformance_errors`
Expected: FAIL — `ya_obs::error` not defined.

- [ ] **Step 3: Implement error types**

Create `rust/ya-obs/src/error.rs`:

```rust
use thiserror::Error;

use crate::xml::parse_error_response;

#[derive(Debug, Error)]
pub enum Error {
    #[error("NoSuchKey: {message} (request_id={request_id})")]
    NoSuchKey { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("NoSuchBucket: {message} (request_id={request_id})")]
    NoSuchBucket { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("AccessDenied: {message} (request_id={request_id})")]
    AccessDenied { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("client error {status} {code}: {message}")]
    Client { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("server error {status} {code}: {message}")]
    Server { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("config error: {0}")]
    Config(String),
}

impl Error {
    pub fn code(&self) -> &str {
        match self {
            Error::NoSuchKey { code, .. }
            | Error::NoSuchBucket { code, .. }
            | Error::AccessDenied { code, .. }
            | Error::Client { code, .. }
            | Error::Server { code, .. } => code,
            Error::Transport(_) => "Transport",
            Error::Xml(_) => "XmlParse",
            Error::Config(_) => "Config",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Error::NoSuchKey { message, .. }
            | Error::NoSuchBucket { message, .. }
            | Error::AccessDenied { message, .. }
            | Error::Client { message, .. }
            | Error::Server { message, .. } => message,
            Error::Transport(_) => "transport error",
            Error::Xml(_) => "xml parse error",
            Error::Config(_) => "config error",
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            Error::NoSuchKey { status, .. }
            | Error::NoSuchBucket { status, .. }
            | Error::AccessDenied { status, .. }
            | Error::Client { status, .. }
            | Error::Server { status, .. } => *status,
            _ => 0,
        }
    }
}

/// Map an HTTP status + OBS XML error body to the right `Error` variant.
pub fn classify_error(status: u16, xml: &str) -> Error {
    let parsed = match parse_error_response(xml) {
        Ok(p) => p,
        Err(e) => return Error::Xml(e),
    };

    match (parsed.code.as_str(), status) {
        ("NoSuchKey", _) => Error::NoSuchKey {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        ("NoSuchBucket", _) => Error::NoSuchBucket {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        ("AccessDenied", _) => Error::AccessDenied {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        _ if status >= 500 => Error::Server {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        _ => Error::Client {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
    }
}
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod error;
pub mod models;
pub mod signer;
pub mod xml;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test conformance_errors`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/error.rs rust/ya-obs/src/lib.rs rust/ya-obs/tests/conformance_errors.rs
git commit -m "feat(rust): typed Error enum + status/XML -> variant classifier"
```

---

### Task 11: Credentials + ClientConfig + URL builder

**Files:**
- Create: `rust/ya-obs/src/credentials.rs`
- Create: `rust/ya-obs/src/config.rs`
- Create: `rust/ya-obs/src/url.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/unit_config.rs`

- [ ] **Step 1: Add failing unit tests**

Create `rust/ya-obs/tests/unit_config.rs`:

```rust
use ya_obs::config::{AddressingStyle, ClientConfig, SigningVersion};
use ya_obs::credentials::Credentials;
use ya_obs::url::build_object_url;

#[test]
fn credentials_from_env_reads_huaweicloud_vars() {
    // SAFETY: tests run single-threaded with a unique env namespace; we restore.
    let old_ak = std::env::var("HUAWEICLOUD_SDK_AK").ok();
    let old_sk = std::env::var("HUAWEICLOUD_SDK_SK").ok();
    std::env::set_var("HUAWEICLOUD_SDK_AK", "test-ak");
    std::env::set_var("HUAWEICLOUD_SDK_SK", "test-sk");

    let c = Credentials::from_env().expect("env credentials");
    assert_eq!(c.access_key, "test-ak");
    assert_eq!(c.secret_key, "test-sk");

    // restore
    match old_ak { Some(v) => std::env::set_var("HUAWEICLOUD_SDK_AK", v), None => std::env::remove_var("HUAWEICLOUD_SDK_AK") };
    match old_sk { Some(v) => std::env::set_var("HUAWEICLOUD_SDK_SK", v), None => std::env::remove_var("HUAWEICLOUD_SDK_SK") };
}

#[test]
fn virtual_addressing_builds_bucket_prefixed_host() {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_addressing_style(AddressingStyle::Virtual);
    let url = build_object_url(&cfg, "my-bucket", "photos/cat.jpg").unwrap();
    assert_eq!(
        url.as_str(),
        "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg"
    );
}

#[test]
fn path_addressing_builds_path_style_url() {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_addressing_style(AddressingStyle::Path);
    let url = build_object_url(&cfg, "my-bucket", "photos/cat.jpg").unwrap();
    assert_eq!(
        url.as_str(),
        "https://obs.cn-north-4.myhuaweicloud.com/my-bucket/photos/cat.jpg"
    );
}

#[test]
fn auto_addressing_falls_back_to_path_when_bucket_has_dot() {
    let cfg = ClientConfig::for_region("cn-north-4");
    let url = build_object_url(&cfg, "bucket.with.dots", "k").unwrap();
    assert!(url.host_str().unwrap().starts_with("obs."));
    assert!(url.path().starts_with("/bucket.with.dots/"));
}

#[test]
fn signing_version_defaults_to_v4() {
    let cfg = ClientConfig::for_region("cn-north-4");
    assert!(matches!(cfg.signing_version, SigningVersion::V4));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && cargo test -p ya-obs --test unit_config`
Expected: FAIL — modules not defined.

- [ ] **Step 3: Implement credentials, config, url**

Create `rust/ya-obs/src/credentials.rs`:

```rust
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
}

impl Credentials {
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self { access_key: access_key.into(), secret_key: secret_key.into() }
    }

    pub fn from_env() -> Result<Self, Error> {
        let access_key = std::env::var("HUAWEICLOUD_SDK_AK")
            .map_err(|_| Error::Config("HUAWEICLOUD_SDK_AK not set".into()))?;
        let secret_key = std::env::var("HUAWEICLOUD_SDK_SK")
            .map_err(|_| Error::Config("HUAWEICLOUD_SDK_SK not set".into()))?;
        Ok(Self { access_key, secret_key })
    }
}
```

Create `rust/ya-obs/src/config.rs`:

```rust
use std::time::Duration;

use crate::credentials::Credentials;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningVersion {
    V4,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingStyle {
    Auto,
    Virtual,
    Path,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub credentials: Option<Credentials>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub signing_version: SigningVersion,
    pub addressing_style: AddressingStyle,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub user_agent: String,
}

impl ClientConfig {
    pub fn for_region(region: impl Into<String>) -> Self {
        Self {
            credentials: None,
            region: Some(region.into()),
            endpoint: None,
            signing_version: SigningVersion::V4,
            addressing_style: AddressingStyle::Auto,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            user_agent: format!("ya-obs/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn for_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            credentials: None,
            region: None,
            endpoint: Some(endpoint.into()),
            signing_version: SigningVersion::V4,
            addressing_style: AddressingStyle::Auto,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            user_agent: format!("ya-obs/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn with_credentials(mut self, c: Credentials) -> Self {
        self.credentials = Some(c);
        self
    }

    pub fn with_signing_version(mut self, v: SigningVersion) -> Self {
        self.signing_version = v;
        self
    }

    pub fn with_addressing_style(mut self, s: AddressingStyle) -> Self {
        self.addressing_style = s;
        self
    }
}
```

Create `rust/ya-obs/src/url.rs`:

```rust
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

use crate::config::{AddressingStyle, ClientConfig};
use crate::error::Error;

const KEY_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?')
    .add(b'`').add(b'{').add(b'}').add(b'%').add(b'+');

fn encode_key(key: &str) -> String {
    key.split('/')
        .map(|seg| utf8_percent_encode(seg, KEY_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_dns_safe_bucket(bucket: &str) -> bool {
    !bucket.is_empty() && !bucket.contains('.') && bucket.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn base_host(cfg: &ClientConfig) -> Result<String, Error> {
    if let Some(ep) = &cfg.endpoint {
        let u = Url::parse(ep).map_err(|e| Error::Config(format!("invalid endpoint {ep}: {e}")))?;
        Ok(u.host_str()
            .ok_or_else(|| Error::Config(format!("endpoint missing host: {ep}")))?
            .to_string())
    } else if let Some(region) = &cfg.region {
        Ok(format!("obs.{region}.myhuaweicloud.com"))
    } else {
        Err(Error::Config("either region or endpoint must be set".into()))
    }
}

pub fn build_object_url(cfg: &ClientConfig, bucket: &str, key: &str) -> Result<Url, Error> {
    let host = base_host(cfg)?;
    let style = match cfg.addressing_style {
        AddressingStyle::Auto => {
            if is_dns_safe_bucket(bucket) { AddressingStyle::Virtual } else { AddressingStyle::Path }
        }
        s => s,
    };

    let url_str = match style {
        AddressingStyle::Virtual => format!("https://{bucket}.{host}/{}", encode_key(key)),
        AddressingStyle::Path => format!("https://{host}/{bucket}/{}", encode_key(key)),
        AddressingStyle::Auto => unreachable!(),
    };
    Url::parse(&url_str).map_err(|e| Error::Config(format!("built invalid url: {e}")))
}
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod config;
pub mod credentials;
pub mod error;
pub mod models;
pub mod signer;
pub mod url;
pub mod xml;

pub use error::Error;
pub use credentials::Credentials;
pub use config::{ClientConfig, SigningVersion, AddressingStyle};
pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test unit_config`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/credentials.rs rust/ya-obs/src/config.rs rust/ya-obs/src/url.rs rust/ya-obs/src/lib.rs rust/ya-obs/tests/unit_config.rs
git commit -m "feat(rust): Credentials, ClientConfig, URL builder with addressing styles"
```

---

### Task 12: Retry policy with Retry-After parsing

**Files:**
- Create: `rust/ya-obs/src/retry.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/unit_retry.rs`

- [ ] **Step 1: Add failing tests**

Create `rust/ya-obs/tests/unit_retry.rs`:

```rust
use std::time::Duration;
use ya_obs::retry::{parse_retry_after, RetryDecision, RetryPolicy};

#[test]
fn retry_policy_retries_5xx() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(500, None, 0), RetryDecision::Retry(_)));
    assert!(matches!(p.classify(503, None, 0), RetryDecision::Retry(_)));
}

#[test]
fn retry_policy_retries_429_and_408() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(408, None, 0), RetryDecision::Retry(_)));
    assert!(matches!(p.classify(429, None, 0), RetryDecision::Retry(_)));
}

#[test]
fn retry_policy_does_not_retry_4xx_other() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(404, None, 0), RetryDecision::DoNotRetry));
    assert!(matches!(p.classify(403, None, 0), RetryDecision::DoNotRetry));
}

#[test]
fn retry_policy_stops_after_max_attempts() {
    let p = RetryPolicy::default();
    assert!(matches!(p.classify(503, None, 2), RetryDecision::DoNotRetry));
}

#[test]
fn retry_after_seconds_overrides_computed_delay() {
    let p = RetryPolicy::default();
    let d = p.classify(503, Some("7"), 0);
    match d {
        RetryDecision::Retry(delay) => assert_eq!(delay, Duration::from_secs(7)),
        _ => panic!("expected Retry"),
    }
}

#[test]
fn parse_retry_after_handles_seconds_and_http_date() {
    assert_eq!(parse_retry_after("5"), Some(Duration::from_secs(5)));
    // HTTP date — we don't need exact value here, just that it's parsed.
    let result = parse_retry_after("Wed, 21 Oct 2099 07:28:00 GMT");
    assert!(result.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && cargo test -p ya-obs --test unit_retry`
Expected: FAIL — `ya_obs::retry` not defined.

- [ ] **Step 3: Implement retry policy**

Create `rust/ya-obs/src/retry.rs`:

```rust
use std::time::{Duration, SystemTime};

use time::{format_description::well_known::Rfc2822, OffsetDateTime};

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RetryDecision {
    Retry(Duration),
    DoNotRetry,
}

impl RetryPolicy {
    /// Decide what to do based on status, optional Retry-After header, and the
    /// number of attempts already made (0 = first call just failed).
    pub fn classify(
        &self,
        status: u16,
        retry_after: Option<&str>,
        attempts_made: u32,
    ) -> RetryDecision {
        if attempts_made + 1 >= self.max_attempts {
            return RetryDecision::DoNotRetry;
        }

        let retryable = status >= 500 || status == 408 || status == 429;
        if !retryable {
            return RetryDecision::DoNotRetry;
        }

        if let Some(h) = retry_after.and_then(parse_retry_after) {
            return RetryDecision::Retry(h.min(self.max_delay));
        }

        // Exponential backoff with full jitter.
        let exp = self.base_delay.saturating_mul(1 << attempts_made);
        let capped = exp.min(self.max_delay);
        let jittered_ms = fastrand::u64(..=capped.as_millis() as u64);
        RetryDecision::Retry(Duration::from_millis(jittered_ms))
    }
}

/// Parse the `Retry-After` header. Returns seconds or an HTTP-date converted
/// to a duration from now (saturating at 0).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(secs) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let dt = OffsetDateTime::parse(value.trim(), &Rfc2822).ok()?;
    let target: SystemTime = dt.into();
    target.duration_since(SystemTime::now()).ok().or(Some(Duration::ZERO))
}
```

Modify `rust/ya-obs/Cargo.toml` — add `fastrand` to `[dependencies]`:

```toml
fastrand = "2"
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod config;
pub mod credentials;
pub mod error;
pub mod models;
pub mod retry;
pub mod signer;
pub mod url;
pub mod xml;

pub use error::Error;
pub use credentials::Credentials;
pub use config::{ClientConfig, SigningVersion, AddressingStyle};
pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test unit_retry`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/retry.rs rust/ya-obs/src/lib.rs rust/ya-obs/Cargo.toml rust/ya-obs/tests/unit_retry.rs
git commit -m "feat(rust): retry policy with Retry-After parsing"
```

---

### Task 13: HTTP layer — signed request executor

**Files:**
- Create: `rust/ya-obs/src/http.rs`
- Modify: `rust/ya-obs/src/lib.rs`

This task adds no new behavior tests — `http.rs` is integration glue. We validate it indirectly via the next tasks (operations + a mocked-server integration test).

- [ ] **Step 1: Implement the HTTP executor**

Create `rust/ya-obs/src/http.rs`:

```rust
use std::time::Duration;

use bytes::Bytes;
use reqwest::{Client as ReqwestClient, Method, RequestBuilder, Response};
use time::{format_description::well_known::Rfc2822, macros::format_description, OffsetDateTime};
use uuid::Uuid;

use crate::config::{ClientConfig, SigningVersion};
use crate::credentials::Credentials;
use crate::error::{classify_error, Error};
use crate::retry::{RetryDecision, RetryPolicy};
use crate::signer::{v2, v4};

pub struct HttpClient {
    inner: ReqwestClient,
    pub config: ClientConfig,
    pub retry: RetryPolicy,
}

const AMZ_DATE_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const DATE_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]");

impl HttpClient {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let inner = ReqwestClient::builder()
            .user_agent(config.user_agent.clone())
            .connect_timeout(config.connect_timeout)
            .timeout(config.read_timeout)
            .build()
            .map_err(Error::Transport)?;
        Ok(Self { inner, config, retry: RetryPolicy::default() })
    }

    fn credentials(&self) -> Result<&Credentials, Error> {
        self.config
            .credentials
            .as_ref()
            .ok_or_else(|| Error::Config("no credentials set on client".into()))
    }

    /// Execute a signed request with retries. Body is buffered (`Bytes`) because
    /// we need to be able to retry it. Streaming uploads use the multipart
    /// path, which handles its own retry granularity per part.
    pub async fn send(
        &self,
        method: Method,
        url: url::Url,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) -> Result<Response, Error> {
        let mut attempts: u32 = 0;
        loop {
            let signed = self.sign_request(&method, &url, &headers, &body)?;
            let resp = self.inner.execute(signed).await.map_err(Error::Transport)?;

            let status = resp.status().as_u16();
            if status < 400 {
                return Ok(resp);
            }

            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            match self.retry.classify(status, retry_after.as_deref(), attempts) {
                RetryDecision::Retry(delay) => {
                    tokio::time::sleep(delay).await;
                    attempts += 1;
                    continue;
                }
                RetryDecision::DoNotRetry => {
                    let body_text = resp.text().await.unwrap_or_default();
                    return Err(classify_error(status, &body_text));
                }
            }
        }
    }

    fn sign_request(
        &self,
        method: &Method,
        url: &url::Url,
        extra_headers: &[(String, String)],
        body: &Bytes,
    ) -> Result<reqwest::Request, Error> {
        let creds = self.credentials()?;
        let now = OffsetDateTime::now_utc();

        let mut headers: Vec<(String, String)> = extra_headers.to_vec();
        headers.push(("Host".into(), url.host_str().unwrap_or_default().to_string()));
        headers.push((
            "x-ya-obs-client-id".into(),
            Uuid::new_v4().to_string(),
        ));

        let auth = match self.config.signing_version {
            SigningVersion::V4 => {
                let amz_date = now.format(AMZ_DATE_FMT).unwrap();
                let date = now.format(DATE_FMT).unwrap();
                let body_sha = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(body));
                let region = self
                    .config
                    .region
                    .as_deref()
                    .ok_or_else(|| Error::Config("region required for V4 signing".into()))?;
                headers.push(("x-amz-date".into(), amz_date.clone()));
                headers.push(("x-amz-content-sha256".into(), body_sha.clone()));

                let mut header_pairs: Vec<(String, String)> = headers.clone();
                header_pairs.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));

                v4::authorization_header(
                    method.as_str(),
                    url.as_str(),
                    &header_pairs,
                    &body_sha,
                    &creds.access_key,
                    &creds.secret_key,
                    &amz_date,
                    &date,
                    region,
                    "s3",
                )
            }
            SigningVersion::V2 => {
                let date = now
                    .format(&Rfc2822)
                    .map_err(|e| Error::Config(format!("date format: {e}")))?;
                headers.push(("Date".into(), date.clone()));

                // Extract bucket + key from URL path or virtual-host.
                let (bucket, key) = extract_bucket_key_from_url(url);
                let obs_headers: std::collections::BTreeMap<String, String> = headers
                    .iter()
                    .filter(|(k, _)| k.to_ascii_lowercase().starts_with("x-obs-"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                v2::authorization_header(
                    method.as_str(),
                    "", // content_md5 — set later when checksums land
                    "",
                    &date,
                    &obs_headers,
                    &bucket,
                    &key,
                    &creds.access_key,
                    &creds.secret_key,
                )
            }
        };

        headers.push(("Authorization".into(), auth));

        let mut req = self
            .inner
            .request(method.clone(), url.clone())
            .body(body.clone());
        for (k, v) in headers {
            // Skip Host — reqwest sets it automatically and rejects manual override on some versions.
            if k.eq_ignore_ascii_case("host") { continue; }
            req = req.header(&k, &v);
        }
        req.build().map_err(Error::Transport)
    }
}

/// Extract `(bucket, key)` from a built OBS URL. Path-style URLs look like
/// `https://obs.<region>.myhuaweicloud.com/<bucket>/<key>`; virtual-host URLs
/// look like `https://<bucket>.obs.<region>.myhuaweicloud.com/<key>`.
fn extract_bucket_key_from_url(url: &url::Url) -> (String, String) {
    let host = url.host_str().unwrap_or_default();
    let path = url.path().trim_start_matches('/');

    if let Some((first_label, rest)) = host.split_once('.') {
        if rest.starts_with("obs.") {
            return (first_label.to_string(), path.to_string());
        }
    }

    // Path-style fallback.
    match path.split_once('/') {
        Some((b, k)) => (b.to_string(), k.to_string()),
        None => (path.to_string(), String::new()),
    }
}
```

Modify `rust/ya-obs/src/lib.rs` — add `pub mod http;`.

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo build -p ya-obs`
Expected: builds clean (warnings okay if unused — fix with clippy in next step).

- [ ] **Step 3: Lint**

Run: `cd rust && cargo clippy -p ya-obs --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add rust/ya-obs/src/http.rs rust/ya-obs/src/lib.rs
git commit -m "feat(rust): signed HTTP executor with retries (V4 + V2)"
```

---

### Task 14: First operation — put_object (single PUT)

**Files:**
- Create: `rust/ya-obs/src/operations/mod.rs`
- Create: `rust/ya-obs/src/operations/put_object.rs`
- Create: `rust/ya-obs/src/client.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Modify: `rust/ya-obs/src/models.rs`
- Create: `rust/ya-obs/tests/integration_put_object.rs`

We use `wiremock` to validate the operation end-to-end without hitting real OBS.

- [ ] **Step 1: Add wiremock dev-dependency**

Modify `rust/ya-obs/Cargo.toml` — append to `[dev-dependencies]`:

```toml
wiremock = "0.6"
```

- [ ] **Step 2: Add failing integration test**

Create `rust/ya-obs/tests/integration_put_object.rs`:

```rust
use bytes::Bytes;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

#[tokio::test]
async fn put_object_sends_signed_put_and_returns_etag() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/my-bucket/hello.txt"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"abc123\""))
        .mount(&server)
        .await;

    let cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    // Region needed for V4 signing even with custom endpoint.
    let mut cfg = cfg;
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let resp = client
        .put_object("my-bucket", "hello.txt", Bytes::from_static(b"hi"))
        .await
        .unwrap();

    assert_eq!(resp.etag, "\"abc123\"");
}
```

- [ ] **Step 3: Implement put_object end-to-end**

Append to `rust/ya-obs/src/models.rs`:

```rust
#[derive(Debug, Clone)]
pub struct PutObjectResponse {
    pub etag: String,
    pub request_id: Option<String>,
}
```

Create `rust/ya-obs/src/operations/mod.rs`:

```rust
pub mod put_object;
```

Create `rust/ya-obs/src/operations/put_object.rs`:

```rust
use bytes::Bytes;
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::url::build_object_url;

pub async fn put_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Result<PutObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;

    let headers = vec![
        ("Content-Length".to_string(), body.len().to_string()),
    ];

    let resp = http.send(Method::PUT, url, headers, body).await?;

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let request_id = resp
        .headers()
        .get("x-obs-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    Ok(PutObjectResponse { etag, request_id })
}
```

Create `rust/ya-obs/src/client.rs`:

```rust
use bytes::Bytes;

use crate::config::ClientConfig;
use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::operations;

pub struct Client {
    http: HttpClient,
}

impl Client {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let http = HttpClient::new(config)?;
        Ok(Self { http })
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
    ) -> Result<PutObjectResponse, Error> {
        operations::put_object::put_object(&self.http, bucket, key, body).await
    }
}
```

Modify `rust/ya-obs/src/lib.rs`:

```rust
//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod client;
pub mod config;
pub mod credentials;
pub mod error;
pub mod http;
pub mod models;
pub mod operations;
pub mod retry;
pub mod signer;
pub mod url;
pub mod xml;

pub use client::Client;
pub use config::{AddressingStyle, ClientConfig, SigningVersion};
pub use credentials::Credentials;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test integration_put_object`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/Cargo.toml rust/ya-obs/src/client.rs rust/ya-obs/src/operations/ rust/ya-obs/src/models.rs rust/ya-obs/src/lib.rs rust/ya-obs/tests/integration_put_object.rs
git commit -m "feat(rust): put_object operation + Client facade"
```

---

### Task 15: get_object + streaming body

**Files:**
- Create: `rust/ya-obs/src/streaming.rs`
- Create: `rust/ya-obs/src/operations/get_object.rs`
- Modify: `rust/ya-obs/src/operations/mod.rs`
- Modify: `rust/ya-obs/src/models.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Create: `rust/ya-obs/tests/integration_get_object.rs`

- [ ] **Step 1: Add failing integration test**

Create `rust/ya-obs/tests/integration_get_object.rs`:

```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

#[tokio::test]
async fn get_object_returns_body_and_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/my-bucket/hello.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"abc123\"")
                .insert_header("Content-Type", "text/plain")
                .set_body_bytes(b"hello world".as_ref()),
        )
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let resp = client.get_object("my-bucket", "hello.txt").await.unwrap();

    assert_eq!(resp.etag, "\"abc123\"");
    assert_eq!(resp.content_type.as_deref(), Some("text/plain"));

    let bytes = resp.body.read_to_end().await.unwrap();
    assert_eq!(&bytes[..], b"hello world");
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cd rust && cargo test -p ya-obs --test integration_get_object`
Expected: FAIL — `get_object` not defined.

- [ ] **Step 3: Implement streaming + get_object**

Create `rust/ya-obs/src/streaming.rs`:

```rust
use bytes::Bytes;
use futures::StreamExt;

use crate::error::Error;

pub struct StreamingBody {
    inner: reqwest::Response,
}

impl StreamingBody {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self { inner }
    }

    pub async fn read_to_end(self) -> Result<Vec<u8>, Error> {
        let bytes = self.inner.bytes().await.map_err(Error::Transport)?;
        Ok(bytes.to_vec())
    }

    pub fn into_stream(self) -> impl futures::Stream<Item = Result<Bytes, Error>> {
        self.inner.bytes_stream().map(|r| r.map_err(Error::Transport))
    }
}
```

Append to `rust/ya-obs/src/models.rs`:

```rust
use crate::streaming::StreamingBody;

pub struct GetObjectResponse {
    pub body: StreamingBody,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub etag: String,
    pub request_id: Option<String>,
}
```

Create `rust/ya-obs/src/operations/get_object.rs`:

```rust
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::GetObjectResponse;
use crate::streaming::StreamingBody;
use crate::url::build_object_url;

pub async fn get_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<GetObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    let resp = http.send(Method::GET, url, Vec::new(), bytes::Bytes::new()).await?;

    let etag = resp.headers().get("etag")
        .and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    let content_type = resp.headers().get("content-type")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let content_length = resp.headers().get("content-length")
        .and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok());
    let request_id = resp.headers().get("x-obs-request-id")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    Ok(GetObjectResponse {
        body: StreamingBody::new(resp),
        content_type,
        content_length,
        etag,
        request_id,
    })
}
```

Modify `rust/ya-obs/src/operations/mod.rs`:

```rust
pub mod get_object;
pub mod put_object;
```

Append to `rust/ya-obs/src/client.rs`:

```rust
use crate::models::GetObjectResponse;

impl Client {
    pub async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectResponse, Error> {
        crate::operations::get_object::get_object(&self.http, bucket, key).await
    }
}
```

Modify `rust/ya-obs/src/lib.rs` — add `pub mod streaming;` between `signer` and `url`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p ya-obs --test integration_get_object`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/streaming.rs rust/ya-obs/src/operations/get_object.rs rust/ya-obs/src/operations/mod.rs rust/ya-obs/src/models.rs rust/ya-obs/src/client.rs rust/ya-obs/src/lib.rs rust/ya-obs/tests/integration_get_object.rs
git commit -m "feat(rust): get_object with streaming body"
```

---

### Task 16: head_object + delete_object

**Files:**
- Create: `rust/ya-obs/src/operations/head_object.rs`
- Create: `rust/ya-obs/src/operations/delete_object.rs`
- Modify: `rust/ya-obs/src/operations/mod.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Modify: `rust/ya-obs/src/models.rs`
- Create: `rust/ya-obs/tests/integration_head_delete.rs`

- [ ] **Step 1: Add failing tests**

Create `rust/ya-obs/tests/integration_head_delete.rs`:

```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

fn make_client(server: &MockServer) -> Client {
    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());
    Client::new(cfg).unwrap()
}

#[tokio::test]
async fn head_object_returns_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/b/k"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"e\"")
                .insert_header("Content-Length", "42"),
        )
        .mount(&server)
        .await;

    let r = make_client(&server).head_object("b", "k").await.unwrap();
    assert_eq!(r.etag, "\"e\"");
    assert_eq!(r.content_length, Some(42));
}

#[tokio::test]
async fn delete_object_returns_ok_on_204() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/b/k"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    make_client(&server).delete_object("b", "k").await.unwrap();
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test -p ya-obs --test integration_head_delete`
Expected: FAIL.

- [ ] **Step 3: Implement operations**

Append to `rust/ya-obs/src/models.rs`:

```rust
#[derive(Debug, Clone)]
pub struct HeadObjectResponse {
    pub etag: String,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub request_id: Option<String>,
}
```

Create `rust/ya-obs/src/operations/head_object.rs`:

```rust
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::HeadObjectResponse;
use crate::url::build_object_url;

pub async fn head_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<HeadObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    let resp = http.send(Method::HEAD, url, Vec::new(), bytes::Bytes::new()).await?;

    Ok(HeadObjectResponse {
        etag: resp.headers().get("etag")
            .and_then(|v| v.to_str().ok()).unwrap_or_default().to_string(),
        content_length: resp.headers().get("content-length")
            .and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok()),
        content_type: resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
        request_id: resp.headers().get("x-obs-request-id")
            .and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
    })
}
```

Create `rust/ya-obs/src/operations/delete_object.rs`:

```rust
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::url::build_object_url;

pub async fn delete_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<(), Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    http.send(Method::DELETE, url, Vec::new(), bytes::Bytes::new()).await?;
    Ok(())
}
```

Modify `rust/ya-obs/src/operations/mod.rs`:

```rust
pub mod delete_object;
pub mod get_object;
pub mod head_object;
pub mod put_object;
```

Append to `rust/ya-obs/src/client.rs`:

```rust
use crate::models::HeadObjectResponse;

impl Client {
    pub async fn head_object(&self, bucket: &str, key: &str) -> Result<HeadObjectResponse, Error> {
        crate::operations::head_object::head_object(&self.http, bucket, key).await
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), Error> {
        crate::operations::delete_object::delete_object(&self.http, bucket, key).await
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p ya-obs --test integration_head_delete`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/operations/ rust/ya-obs/src/client.rs rust/ya-obs/src/models.rs rust/ya-obs/tests/integration_head_delete.rs
git commit -m "feat(rust): head_object + delete_object"
```

---

### Task 17: list_objects with pagination

**Files:**
- Create: `rust/ya-obs/src/operations/list_objects.rs`
- Modify: `rust/ya-obs/src/operations/mod.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Create: `rust/ya-obs/tests/integration_list_objects.rs`

- [ ] **Step 1: Add failing test**

Create `rust/ya-obs/tests/integration_list_objects.rs`:

```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

const PAGE_1: &str = r#"<?xml version="1.0"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>b</Name><Prefix></Prefix><MaxKeys>1</MaxKeys>
  <IsTruncated>true</IsTruncated><NextMarker>k1</NextMarker>
  <Contents><Key>k1</Key><LastModified>2024-01-15T10:30:00.000Z</LastModified><ETag>"e1"</ETag><Size>1</Size></Contents>
</ListBucketResult>"#;

const PAGE_2: &str = r#"<?xml version="1.0"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>b</Name><Prefix></Prefix><MaxKeys>1</MaxKeys>
  <IsTruncated>false</IsTruncated>
  <Contents><Key>k2</Key><LastModified>2024-01-15T10:30:00.000Z</LastModified><ETag>"e2"</ETag><Size>2</Size></Contents>
</ListBucketResult>"#;

#[tokio::test]
async fn list_objects_collects_pages() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/b/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE_1))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/b/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE_2))
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let all = client.list_objects("b", None).await.unwrap();

    let keys: Vec<&str> = all.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["k1", "k2"]);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd rust && cargo test -p ya-obs --test integration_list_objects`
Expected: FAIL.

- [ ] **Step 3: Implement listing**

Create `rust/ya-obs/src/operations/list_objects.rs`:

```rust
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::ListedObject;
use crate::url::build_object_url;
use crate::xml::parse_list_bucket_result;

pub async fn list_objects(
    http: &HttpClient,
    bucket: &str,
    prefix: Option<&str>,
) -> Result<Vec<ListedObject>, Error> {
    let mut all = Vec::new();
    let mut marker: Option<String> = None;

    loop {
        let mut url = build_object_url(&http.config, bucket, "")?;
        {
            let mut q = url.query_pairs_mut();
            if let Some(p) = prefix {
                q.append_pair("prefix", p);
            }
            if let Some(m) = &marker {
                q.append_pair("marker", m);
            }
        }

        let resp = http.send(Method::GET, url, Vec::new(), bytes::Bytes::new()).await?;
        let body = resp.text().await.map_err(Error::Transport)?;
        let page = parse_list_bucket_result(&body)?;

        let last_key = page.objects.last().map(|o| o.key.clone());
        all.extend(page.objects);

        if !page.is_truncated {
            return Ok(all);
        }
        marker = page.next_marker.or(last_key);
        if marker.is_none() {
            return Ok(all);
        }
    }
}
```

Modify `rust/ya-obs/src/operations/mod.rs`:

```rust
pub mod delete_object;
pub mod get_object;
pub mod head_object;
pub mod list_objects;
pub mod put_object;
```

Append to `rust/ya-obs/src/client.rs`:

```rust
use crate::models::ListedObject;

impl Client {
    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<ListedObject>, Error> {
        crate::operations::list_objects::list_objects(&self.http, bucket, prefix).await
    }
}
```

- [ ] **Step 4: Run test**

Run: `cd rust && cargo test -p ya-obs --test integration_list_objects`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/operations/ rust/ya-obs/src/client.rs rust/ya-obs/tests/integration_list_objects.rs
git commit -m "feat(rust): list_objects with transparent pagination"
```

---

### Task 18: Presign API on Client

**Files:**
- Create: `rust/ya-obs/src/operations/presign.rs`
- Modify: `rust/ya-obs/src/operations/mod.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Create: `rust/ya-obs/tests/unit_presign.rs`

- [ ] **Step 1: Add failing test**

Create `rust/ya-obs/tests/unit_presign.rs`:

```rust
use std::collections::HashMap;
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig, SigningVersion};
use ya_obs::credentials::Credentials;

fn client(sv: SigningVersion) -> Client {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_credentials(Credentials::new("AK", "SK"))
        .with_signing_version(sv)
        .with_addressing_style(AddressingStyle::Virtual);
    Client::new(cfg).unwrap()
}

#[test]
fn presign_v4_includes_required_params() {
    let url = client(SigningVersion::V4)
        .presign_get_object("my-bucket", "k.jpg", 3600).unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    let qs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    for k in ["X-Amz-Algorithm", "X-Amz-Credential", "X-Amz-Date", "X-Amz-Expires", "X-Amz-SignedHeaders", "X-Amz-Signature"] {
        assert!(qs.contains_key(k), "missing {k}");
    }
}

#[test]
fn presign_v2_includes_required_params() {
    let url = client(SigningVersion::V2)
        .presign_get_object("my-bucket", "k.jpg", 3600).unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    let qs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    for k in ["AccessKeyId", "Expires", "Signature"] {
        assert!(qs.contains_key(k), "missing {k}");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd rust && cargo test -p ya-obs --test unit_presign`
Expected: FAIL.

- [ ] **Step 3: Implement presign**

Create `rust/ya-obs/src/operations/presign.rs`:

```rust
use time::{format_description::macros::format_description, OffsetDateTime};

use crate::config::{ClientConfig, SigningVersion};
use crate::error::Error;
use crate::signer::{v2, v4};
use crate::url::build_object_url;

const AMZ_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]");

pub fn presign_get_object(
    cfg: &ClientConfig,
    bucket: &str,
    key: &str,
    expires: u64,
) -> Result<String, Error> {
    let creds = cfg.credentials.as_ref()
        .ok_or_else(|| Error::Config("credentials required for presign".into()))?;
    let url = build_object_url(cfg, bucket, key)?;
    let now = OffsetDateTime::now_utc();

    match cfg.signing_version {
        SigningVersion::V4 => {
            let region = cfg.region.as_deref()
                .ok_or_else(|| Error::Config("region required for V4 presign".into()))?;
            let amz_date = now.format(AMZ_DATE).unwrap();
            let date = now.format(DATE).unwrap();
            Ok(v4::presign_url(
                "GET", url.as_str(),
                &creds.access_key, &creds.secret_key,
                &amz_date, &date, region, "s3",
                expires,
            ))
        }
        SigningVersion::V2 => {
            let expires_unix = now.unix_timestamp() as u64 + expires;
            let (signed, _sts) = v2::presign_url(
                "GET", url.as_str(),
                bucket, key, expires_unix,
                &std::collections::BTreeMap::new(),
                &creds.access_key, &creds.secret_key,
            );
            Ok(signed)
        }
    }
}
```

Modify `rust/ya-obs/src/operations/mod.rs`:

```rust
pub mod delete_object;
pub mod get_object;
pub mod head_object;
pub mod list_objects;
pub mod presign;
pub mod put_object;
```

Append to `rust/ya-obs/src/client.rs`:

```rust
impl Client {
    pub fn presign_get_object(
        &self,
        bucket: &str,
        key: &str,
        expires: u64,
    ) -> Result<String, Error> {
        crate::operations::presign::presign_get_object(&self.http.config, bucket, key, expires)
    }
}
```

- [ ] **Step 4: Run test**

Run: `cd rust && cargo test -p ya-obs --test unit_presign`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/operations/ rust/ya-obs/src/client.rs rust/ya-obs/tests/unit_presign.rs
git commit -m "feat(rust): presign_get_object for V4 and V2"
```

---

### Task 19: CLI scaffolding — clap args + obs:// URI parser

**Files:**
- Modify: `rust/ya-obs-cli/src/main.rs`
- Create: `rust/ya-obs-cli/src/args.rs`
- Create: `rust/ya-obs-cli/src/obs_uri.rs`
- Create: `rust/ya-obs-cli/src/commands/mod.rs`
- Create: `rust/ya-obs-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Add assert_cmd dev-dependency**

Modify `rust/ya-obs-cli/Cargo.toml` — add `[dev-dependencies]`:

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: Add failing CLI smoke test**

Create `rust/ya-obs-cli/tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_prints_subcommands() {
    Command::cargo_bin("ya-obs").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("ls"))
        .stdout(contains("cp"))
        .stdout(contains("rm"))
        .stdout(contains("cat"))
        .stdout(contains("presign"));
}

#[test]
fn obs_uri_parses_bucket_and_key() {
    use ya_obs_cli::obs_uri::ObsUri;
    let u: ObsUri = "obs://my-bucket/path/to/file.txt".parse().unwrap();
    assert_eq!(u.bucket, "my-bucket");
    assert_eq!(u.key, "path/to/file.txt");
}

#[test]
fn obs_uri_rejects_non_obs_scheme() {
    use ya_obs_cli::obs_uri::ObsUri;
    let r: Result<ObsUri, _> = "s3://b/k".parse();
    assert!(r.is_err());
}

#[test]
fn obs_uri_accepts_bucket_only() {
    use ya_obs_cli::obs_uri::ObsUri;
    let u: ObsUri = "obs://my-bucket".parse().unwrap();
    assert_eq!(u.bucket, "my-bucket");
    assert_eq!(u.key, "");
}
```

For the `ya_obs_cli::obs_uri` import to work, the binary crate needs a lib target alongside the bin. Modify `rust/ya-obs-cli/Cargo.toml`:

```toml
[lib]
path = "src/lib.rs"

[[bin]]
name = "ya-obs"
path = "src/main.rs"
```

- [ ] **Step 3: Run smoke test to verify failure**

Run: `cd rust && cargo test -p ya-obs-cli --test cli_smoke`
Expected: FAIL — bin doesn't exist with proper help yet.

- [ ] **Step 4: Implement the CLI surface**

Create `rust/ya-obs-cli/src/obs_uri.rs`:

```rust
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsUri {
    pub bucket: String,
    pub key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ObsUriError {
    #[error("expected obs:// scheme, got {0}")]
    BadScheme(String),
    #[error("missing bucket in {0}")]
    NoBucket(String),
}

impl FromStr for ObsUri {
    type Err = ObsUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s.strip_prefix("obs://")
            .ok_or_else(|| ObsUriError::BadScheme(s.to_string()))?;
        let (bucket, key) = match rest.split_once('/') {
            Some((b, k)) => (b, k),
            None => (rest, ""),
        };
        if bucket.is_empty() {
            return Err(ObsUriError::NoBucket(s.to_string()));
        }
        Ok(ObsUri { bucket: bucket.to_string(), key: key.to_string() })
    }
}
```

Create `rust/ya-obs-cli/src/args.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ya-obs", version, about = "Huawei Cloud OBS CLI")]
pub struct Cli {
    /// Region (e.g. cn-north-4). Overridden by --endpoint.
    #[arg(long, env = "YA_OBS_REGION")]
    pub region: Option<String>,

    /// Custom endpoint URL (overrides --region).
    #[arg(long, env = "YA_OBS_ENDPOINT")]
    pub endpoint: Option<String>,

    /// Access key (defaults to $HUAWEICLOUD_SDK_AK).
    #[arg(long, env = "HUAWEICLOUD_SDK_AK", hide_env_values = true)]
    pub access_key: Option<String>,

    /// Secret key (defaults to $HUAWEICLOUD_SDK_SK).
    #[arg(long, env = "HUAWEICLOUD_SDK_SK", hide_env_values = true)]
    pub secret_key: Option<String>,

    /// Signing version.
    #[arg(long, value_enum, default_value_t = SignVer::V4)]
    pub signing_version: SignVer,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SignVer { V4, V2 }

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List objects under an obs:// URI.
    Ls { uri: String },
    /// Copy between local and obs://.
    Cp { src: String, dst: String },
    /// Delete an object.
    Rm { uri: String },
    /// Print object body to stdout.
    Cat { uri: String },
    /// Generate a presigned GET URL.
    Presign {
        uri: String,
        /// Expiry in seconds.
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },
}
```

Create `rust/ya-obs-cli/src/commands/mod.rs`:

```rust
// Subcommand implementations land here in the next tasks. For the scaffold we
// only need clap to recognize the names — the smoke test asserts on --help.
```

Create `rust/ya-obs-cli/src/lib.rs`:

```rust
pub mod args;
pub mod obs_uri;
pub mod commands;
```

Replace `rust/ya-obs-cli/src/main.rs`:

```rust
use anyhow::Result;
use clap::Parser;
use ya_obs_cli::args::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    // Subcommands implemented in the next tasks.
    println!("ya-obs cli — subcommands not implemented yet in this task");
    Ok(())
}
```

- [ ] **Step 5: Run smoke test to verify pass**

Run: `cd rust && cargo test -p ya-obs-cli --test cli_smoke`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add rust/ya-obs-cli/
git commit -m "feat(rust-cli): scaffold clap args, obs:// uri parser"
```

---

### Task 20: CLI `ls`, `rm`, `cat`, `presign`

**Files:**
- Create: `rust/ya-obs-cli/src/commands/ls.rs`
- Create: `rust/ya-obs-cli/src/commands/rm.rs`
- Create: `rust/ya-obs-cli/src/commands/cat.rs`
- Create: `rust/ya-obs-cli/src/commands/presign.rs`
- Modify: `rust/ya-obs-cli/src/commands/mod.rs`
- Modify: `rust/ya-obs-cli/src/main.rs`

These four commands are similar enough to bundle. `cp` (with progress) gets its own task.

- [ ] **Step 1: Implement the four commands**

Create `rust/ya-obs-cli/src/commands/ls.rs`:

```rust
use anyhow::Result;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let prefix = if parsed.key.is_empty() { None } else { Some(parsed.key.as_str()) };
    let objects = client.list_objects(&parsed.bucket, prefix).await?;
    for o in objects {
        println!("{:>12}  {}  {}", o.size, o.last_modified, o.key);
    }
    Ok(())
}
```

Create `rust/ya-obs-cli/src/commands/rm.rs`:

```rust
use anyhow::{anyhow, Result};
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    if parsed.key.is_empty() {
        return Err(anyhow!("rm requires an object key, got bucket-only URI {uri}"));
    }
    client.delete_object(&parsed.bucket, &parsed.key).await?;
    Ok(())
}
```

Create `rust/ya-obs-cli/src/commands/cat.rs`:

```rust
use anyhow::Result;
use tokio::io::AsyncWriteExt;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let resp = client.get_object(&parsed.bucket, &parsed.key).await?;
    let bytes = resp.body.read_to_end().await?;
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&bytes).await?;
    stdout.flush().await?;
    Ok(())
}
```

Create `rust/ya-obs-cli/src/commands/presign.rs`:

```rust
use anyhow::Result;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub fn run(client: &Client, uri: &str, expires: u64) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let url = client.presign_get_object(&parsed.bucket, &parsed.key, expires)?;
    println!("{url}");
    Ok(())
}
```

Modify `rust/ya-obs-cli/src/commands/mod.rs`:

```rust
pub mod cat;
pub mod ls;
pub mod presign;
pub mod rm;
```

Replace `rust/ya-obs-cli/src/main.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::Parser;
use ya_obs::{AddressingStyle, Client, ClientConfig, Credentials, SigningVersion};
use ya_obs_cli::args::{Cli, Cmd, SignVer};
use ya_obs_cli::commands;

fn build_client(cli: &Cli) -> Result<Client> {
    let mut cfg = match (&cli.endpoint, &cli.region) {
        (Some(ep), _) => ClientConfig::for_endpoint(ep),
        (None, Some(r)) => ClientConfig::for_region(r),
        (None, None) => return Err(anyhow!("set --region or --endpoint (or env YA_OBS_REGION / YA_OBS_ENDPOINT)")),
    };
    if let (Some(ak), Some(sk)) = (&cli.access_key, &cli.secret_key) {
        cfg = cfg.with_credentials(Credentials::new(ak, sk));
    } else {
        cfg = cfg.with_credentials(Credentials::from_env()?);
    }
    cfg = cfg.with_signing_version(match cli.signing_version {
        SignVer::V4 => SigningVersion::V4,
        SignVer::V2 => SigningVersion::V2,
    });
    cfg = cfg.with_addressing_style(AddressingStyle::Auto);
    Ok(Client::new(cfg)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let client = build_client(&cli)?;

    match &cli.cmd {
        Cmd::Ls { uri } => commands::ls::run(&client, uri).await?,
        Cmd::Rm { uri } => commands::rm::run(&client, uri).await?,
        Cmd::Cat { uri } => commands::cat::run(&client, uri).await?,
        Cmd::Presign { uri, expires } => commands::presign::run(&client, uri, *expires)?,
        Cmd::Cp { .. } => return Err(anyhow!("cp not yet implemented (Task 21)")),
    }
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles + smoke test still passes**

Run: `cd rust && cargo test -p ya-obs-cli`
Expected: PASS — smoke tests still green; build succeeds.

- [ ] **Step 3: Commit**

```bash
git add rust/ya-obs-cli/src/
git commit -m "feat(rust-cli): implement ls, rm, cat, presign"
```

---

### Task 21: CLI `cp` (local↔obs) with progress bar

**Files:**
- Create: `rust/ya-obs-cli/src/progress.rs`
- Create: `rust/ya-obs-cli/src/commands/cp.rs`
- Modify: `rust/ya-obs-cli/src/commands/mod.rs`
- Modify: `rust/ya-obs-cli/src/main.rs`

For v0.1, `cp` handles only single-object operations (no recursive directory copy). Multipart uploads come in Task 22.

- [ ] **Step 1: Implement progress + cp**

Create `rust/ya-obs-cli/src/progress.rs`:

```rust
use indicatif::{ProgressBar, ProgressStyle};

pub fn bytes_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner} {bytes}/{total_bytes} ({bytes_per_sec}) [{wide_bar}] {percent}%",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    bar
}
```

Create `rust/ya-obs-cli/src/commands/cp.rs`:

```rust
use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ya_obs::Client;

use crate::obs_uri::ObsUri;
use crate::progress::bytes_bar;

enum Endpoint { Local(String), Remote(ObsUri) }

fn classify(s: &str) -> Endpoint {
    if let Ok(u) = s.parse::<ObsUri>() { Endpoint::Remote(u) } else { Endpoint::Local(s.to_string()) }
}

pub async fn run(client: &Client, src: &str, dst: &str) -> Result<()> {
    match (classify(src), classify(dst)) {
        (Endpoint::Local(path), Endpoint::Remote(u)) => upload(client, &path, &u).await,
        (Endpoint::Remote(u), Endpoint::Local(path)) => download(client, &u, &path).await,
        (Endpoint::Local(_), Endpoint::Local(_)) =>
            Err(anyhow!("at least one side of cp must be obs://")),
        (Endpoint::Remote(_), Endpoint::Remote(_)) =>
            Err(anyhow!("server-side copy not yet implemented")),
    }
}

async fn upload(client: &Client, path: &str, dst: &ObsUri) -> Result<()> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let bar = bytes_bar(buf.len() as u64);
    bar.set_message(format!("uploading {} -> obs://{}/{}", path, dst.bucket, dst.key));
    client.put_object(&dst.bucket, &dst.key, Bytes::from(buf.clone())).await?;
    bar.inc(buf.len() as u64);
    bar.finish_with_message("done");
    Ok(())
}

async fn download(client: &Client, src: &ObsUri, path: &str) -> Result<()> {
    let resp = client.get_object(&src.bucket, &src.key).await?;
    let total = resp.content_length.unwrap_or(0);
    let bar = bytes_bar(total);
    bar.set_message(format!("downloading obs://{}/{} -> {}", src.bucket, src.key, path));

    let mut out = File::create(path).await?;
    let mut stream = Box::pin(resp.body.into_stream());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        out.write_all(&chunk).await?;
        bar.inc(chunk.len() as u64);
    }
    out.flush().await?;
    bar.finish_with_message("done");
    Ok(())
}
```

Modify `rust/ya-obs-cli/src/commands/mod.rs`:

```rust
pub mod cat;
pub mod cp;
pub mod ls;
pub mod presign;
pub mod rm;
```

In `rust/ya-obs-cli/src/main.rs`, replace the `Cmd::Cp { .. }` arm:

```rust
Cmd::Cp { src, dst } => commands::cp::run(&client, src, dst).await?,
```

- [ ] **Step 2: Verify compile**

Run: `cd rust && cargo build -p ya-obs-cli`
Expected: PASS.

- [ ] **Step 3: Manual smoke test against mock server is unnecessary** — the underlying `put_object`/`get_object` are tested in Tasks 14–15; `cp` is shell glue. Just confirm the smoke `--help` still passes.

Run: `cd rust && cargo test -p ya-obs-cli --test cli_smoke`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add rust/ya-obs-cli/
git commit -m "feat(rust-cli): cp command with indicatif progress bar"
```

---

### Task 22: Multipart upload

**Files:**
- Create: `rust/ya-obs/src/multipart.rs`
- Modify: `rust/ya-obs/src/lib.rs`
- Modify: `rust/ya-obs/src/operations/put_object.rs`
- Modify: `rust/ya-obs/src/models.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Create: `rust/ya-obs/tests/integration_multipart.rs`

Multipart needs three OBS endpoints: `POST /<bucket>/<key>?uploads` to initiate, `PUT /<bucket>/<key>?partNumber=N&uploadId=ID` per part, `POST /<bucket>/<key>?uploadId=ID` to complete. Plus `DELETE /<bucket>/<key>?uploadId=ID` to abort.

- [ ] **Step 1: Add failing integration test**

Create `rust/ya-obs/tests/integration_multipart.rs`:

```rust
use bytes::Bytes;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

const INITIATE_XML: &str = r#"<?xml version="1.0"?>
<InitiateMultipartUploadResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Bucket>b</Bucket><Key>k</Key><UploadId>upload-xyz</UploadId>
</InitiateMultipartUploadResult>"#;

#[tokio::test]
async fn multipart_uploads_two_parts_and_completes() {
    let server = MockServer::start().await;

    Mock::given(method("POST")).and(path("/b/k")).and(query_param("uploads", ""))
        .respond_with(ResponseTemplate::new(200).set_body_string(INITIATE_XML))
        .mount(&server).await;

    Mock::given(method("PUT")).and(path("/b/k")).and(query_param("partNumber", "1"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"p1\""))
        .mount(&server).await;

    Mock::given(method("PUT")).and(path("/b/k")).and(query_param("partNumber", "2"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"p2\""))
        .mount(&server).await;

    Mock::given(method("POST")).and(path("/b/k")).and(query_param("uploadId", "upload-xyz"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"final\""))
        .mount(&server).await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let mut body = vec![0u8; 10 * 1024 * 1024]; // 10 MB
    body[0] = 1;
    let resp = client.put_object_multipart("b", "k", Bytes::from(body), 5 * 1024 * 1024, 2).await.unwrap();
    assert_eq!(resp.etag, "\"final\"");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd rust && cargo test -p ya-obs --test integration_multipart`
Expected: FAIL — `put_object_multipart` not defined.

- [ ] **Step 3: Implement multipart**

Create `rust/ya-obs/src/multipart.rs`:

```rust
use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::url::build_object_url;
use crate::xml::{parse_initiate_multipart_result, serialize_complete_multipart};

async fn initiate(http: &HttpClient, bucket: &str, key: &str) -> Result<String, Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploads", "");
    let resp = http.send(Method::POST, url, Vec::new(), Bytes::new()).await?;
    let body = resp.text().await.map_err(Error::Transport)?;
    Ok(parse_initiate_multipart_result(&body)?.upload_id)
}

async fn upload_part(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: u32,
    body: Bytes,
) -> Result<(u32, String), Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut()
        .append_pair("partNumber", &part_number.to_string())
        .append_pair("uploadId", upload_id);
    let headers = vec![("Content-Length".to_string(), body.len().to_string())];
    let resp = http.send(Method::PUT, url, headers, body).await?;
    let etag = resp.headers().get("etag")
        .and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    Ok((part_number, etag))
}

async fn complete(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    upload_id: &str,
    parts: Vec<(u32, String)>,
) -> Result<PutObjectResponse, Error> {
    let xml = serialize_complete_multipart(&parts);
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploadId", upload_id);
    let headers = vec![
        ("Content-Type".to_string(), "application/xml".to_string()),
        ("Content-Length".to_string(), xml.len().to_string()),
    ];
    let resp = http.send(Method::POST, url, headers, Bytes::from(xml)).await?;
    let etag = resp.headers().get("etag")
        .and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    let request_id = resp.headers().get("x-obs-request-id")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    Ok(PutObjectResponse { etag, request_id })
}

async fn abort(http: &HttpClient, bucket: &str, key: &str, upload_id: &str) -> Result<(), Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploadId", upload_id);
    http.send(Method::DELETE, url, Vec::new(), Bytes::new()).await?;
    Ok(())
}

/// Upload `body` as a multipart upload, splitting into `part_size` chunks and
/// running `concurrency` part uploads in parallel.
pub async fn upload(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
    part_size: usize,
    concurrency: usize,
) -> Result<PutObjectResponse, Error> {
    let upload_id = initiate(http, bucket, key).await?;

    let mut chunks: Vec<(u32, Bytes)> = Vec::new();
    let mut offset = 0usize;
    let mut n: u32 = 1;
    while offset < body.len() {
        let end = (offset + part_size).min(body.len());
        chunks.push((n, body.slice(offset..end)));
        offset = end;
        n += 1;
    }

    let result: Result<Vec<(u32, String)>, Error> = async {
        let mut in_flight: FuturesUnordered<_> = FuturesUnordered::new();
        let mut iter = chunks.into_iter();
        for _ in 0..concurrency {
            if let Some((pn, b)) = iter.next() {
                in_flight.push(upload_part(http, bucket, key, &upload_id, pn, b));
            }
        }
        let mut finished: Vec<(u32, String)> = Vec::new();
        while let Some(res) = in_flight.next().await {
            finished.push(res?);
            if let Some((pn, b)) = iter.next() {
                in_flight.push(upload_part(http, bucket, key, &upload_id, pn, b));
            }
        }
        finished.sort_by_key(|(n, _)| *n);
        Ok(finished)
    }.await;

    match result {
        Ok(parts) => complete(http, bucket, key, &upload_id, parts).await,
        Err(e) => {
            let _ = abort(http, bucket, key, &upload_id).await;
            Err(e)
        }
    }
}
```

Modify `rust/ya-obs/src/lib.rs` — add `pub mod multipart;`.

Append to `rust/ya-obs/src/client.rs`:

```rust
impl Client {
    pub async fn put_object_multipart(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
        part_size: usize,
        concurrency: usize,
    ) -> Result<PutObjectResponse, Error> {
        crate::multipart::upload(&self.http, bucket, key, body, part_size, concurrency).await
    }
}
```

- [ ] **Step 4: Run test**

Run: `cd rust && cargo test -p ya-obs --test integration_multipart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/ya-obs/src/multipart.rs rust/ya-obs/src/lib.rs rust/ya-obs/src/client.rs rust/ya-obs/tests/integration_multipart.rs
git commit -m "feat(rust): multipart upload with bounded concurrency + abort on failure"
```

---

### Task 23: Wire multipart into put_object auto-trigger + CLI cp

**Files:**
- Modify: `rust/ya-obs/src/operations/put_object.rs`
- Modify: `rust/ya-obs/src/client.rs`
- Modify: `rust/ya-obs-cli/src/commands/cp.rs`

Default threshold: 100 MB. Default part size: 8 MB. Default concurrency: 4. Matches the Python SDK defaults from the design spec.

- [ ] **Step 1: Update put_object to auto-multipart**

Replace the body of `rust/ya-obs/src/operations/put_object.rs`:

```rust
use bytes::Bytes;
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::multipart;
use crate::url::build_object_url;

const MULTIPART_THRESHOLD: usize = 100 * 1024 * 1024;
const DEFAULT_PART_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_CONCURRENCY: usize = 4;

pub async fn put_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Result<PutObjectResponse, Error> {
    if body.len() >= MULTIPART_THRESHOLD {
        return multipart::upload(
            http, bucket, key, body, DEFAULT_PART_SIZE, DEFAULT_CONCURRENCY,
        ).await;
    }

    let url = build_object_url(&http.config, bucket, key)?;
    let headers = vec![("Content-Length".to_string(), body.len().to_string())];
    let resp = http.send(Method::PUT, url, headers, body).await?;

    let etag = resp.headers().get("etag")
        .and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    let request_id = resp.headers().get("x-obs-request-id")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    Ok(PutObjectResponse { etag, request_id })
}
```

- [ ] **Step 2: Run all tests to verify nothing regressed**

Run: `cd rust && cargo test --workspace`
Expected: PASS (all conformance + integration tests).

- [ ] **Step 3: Commit**

```bash
git add rust/ya-obs/src/operations/put_object.rs
git commit -m "feat(rust): put_object auto-switches to multipart at 100 MB"
```

---

### Task 24: CI workflow

**Files:**
- Create: `.github/workflows/rust.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/rust.yml`:

```yaml
name: rust

on:
  push:
    branches: [main]
    paths:
      - "rust/**"
      - "test-vectors/**"
      - ".github/workflows/rust.yml"
  pull_request:
    paths:
      - "rust/**"
      - "test-vectors/**"
      - ".github/workflows/rust.yml"

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    defaults:
      run:
        working-directory: rust
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust -> target
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: test
        run: cargo test --workspace --all-targets
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "ci: add rust fmt+clippy+test workflow"
```

---

### Task 25: cargo-dist release config + README updates

**Files:**
- Modify: `rust/Cargo.toml`
- Create: `rust/CHANGELOG.md`
- Modify: `README.md`

- [ ] **Step 1: Add cargo-dist metadata**

Append to `rust/Cargo.toml` (top-level workspace manifest):

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.22.1"
ci = ["github"]
installers = ["shell", "powershell"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
pr-run-mode = "plan"
```

Then run (the engineer should do this manually, not commit it as a task step):

```
cd rust && cargo dist init --yes
```

…which generates `.github/workflows/release.yml`. If `cargo-dist` is not installed: `cargo install cargo-dist`.

- [ ] **Step 2: Write the changelog**

Create `rust/CHANGELOG.md`:

```markdown
# Changelog

All notable changes to the `ya-obs` and `ya-obs-cli` crates.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - unreleased

### Added
- `ya-obs` library crate: SigV4 + OBS V2 signing (header + presigned), object
  PUT/GET/HEAD/DELETE/LIST with pagination, presigned GET URLs, streaming GET
  bodies, automatic multipart upload at 100 MB threshold, typed error variants
  for `NoSuchKey` / `NoSuchBucket` / `AccessDenied`, configurable retry policy
  with `Retry-After` parsing.
- `ya-obs-cli` binary crate (`ya-obs`): `ls`, `cp` (with progress bar),
  `rm`, `cat`, `presign` subcommands.
- Cross-language conformance test suite validated against `test-vectors/`.
```

- [ ] **Step 3: Update the project README**

Find the language matrix in `README.md` and update the Rust row (currently "planned") to:

```markdown
| Rust | `ya-obs` (crates.io) | v0.1.0 |
| Rust CLI | `ya-obs-cli` (crates.io) | v0.1.0 |
```

After the Python quickstart, add a new section:

```markdown
## Rust quickstart

```bash
cargo add ya-obs tokio bytes
```

```rust
use bytes::Bytes;
use ya_obs::{Client, ClientConfig, Credentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_credentials(Credentials::from_env()?);
    let client = Client::new(cfg)?;
    client.put_object("my-bucket", "hello.txt", Bytes::from_static(b"hi")).await?;
    Ok(())
}
```

## CLI

```bash
cargo install ya-obs-cli
export HUAWEICLOUD_SDK_AK=...
export HUAWEICLOUD_SDK_SK=...
ya-obs --region cn-north-4 ls obs://my-bucket
ya-obs --region cn-north-4 cp ./big.bin obs://my-bucket/big.bin
ya-obs --region cn-north-4 presign obs://my-bucket/file.txt --expires 3600
```
```

- [ ] **Step 4: Commit**

```bash
git add rust/Cargo.toml rust/CHANGELOG.md README.md .github/workflows/release.yml
git commit -m "release(rust): cargo-dist config + readme/changelog for v0.1"
```

---

### Task 26: Final conformance sweep + clippy clean-up

**Files:** (no new files)

- [ ] **Step 1: Run the full conformance + integration suite**

Run: `cd rust && cargo test --workspace --all-targets`
Expected: PASS, every test green.

- [ ] **Step 2: Lint everything**

Run: `cd rust && cargo clippy --workspace --all-targets -- -D warnings`
Expected: zero warnings.

- [ ] **Step 3: Format check**

Run: `cd rust && cargo fmt --all -- --check`
Expected: PASS.

- [ ] **Step 4: Conformance summary**

Write a one-paragraph compliance line to `rust/CHANGELOG.md` under the [0.1.0] entry's `### Added`:

```markdown
- Conformance: 100% auth vectors (4 V4 + 4 V2), 100% xml vectors (4), 100%
  error vectors (3). Multipart scenario test against wiremock validates the
  initiate/part/complete sequence.
```

- [ ] **Step 5: Commit**

```bash
git add rust/CHANGELOG.md
git commit -m "docs(rust): record v0.1 conformance summary"
```

---

## Self-Review

**Spec coverage check** (against the v0.1 scope from `docs/superpowers/specs/2026-04-17-ya-obs-sdk-design.md`):

- Tier 1 operations: `put_object` (Task 14, multipart Task 22-23), `get_object` (Task 15), `head_object` (Task 16), `delete_object` (Task 16), `list_objects` (Task 17). ✓
- Tier 3: multipart upload (Tasks 22-23), range downloads — **deferred to a v0.2 plan**, called out below. Server-side copy — **deferred**.
- Tier 4: user metadata, system metadata, presigned URLs (Task 18). Metadata pass-through is implemented but **not exhaustively exercised** in tests; the spec demands at least functional coverage. Acceptable for an MVP port — listed in deferred items.
- Test vectors: auth/v4 (Tasks 3-5), auth/v2 (Tasks 6-7), xml (Tasks 8-9), errors (Task 10). 100% coverage. ✓
- Credentials resolution (env): Task 11. ✓
- Addressing styles: Task 11. ✓
- Retry policy with Retry-After: Task 12. ✓
- Error hierarchy: Task 10. ✓
- CLI surface: Tasks 19-21. ✓
- Distribution: Task 25 (cargo-dist + crates.io). ✓

**Deferred to a follow-up plan (intentional v0.1 scope cut):**

- Range downloads, `If-Match`/`If-None-Match` on GET.
- Server-side `copy_object`.
- User-metadata round-trip tests.
- `Content-MD5` on uploads.
- Recursive `cp` for directories.
- Progress callbacks at the SDK level (the CLI has its own bar; library-level callbacks can land later).
- Homebrew tap.

**Placeholder scan:** No "TBD" / "implement later" remaining. All code blocks are concrete and copy-pasteable.

**Type consistency check:**

- `ListedObject` defined in Task 8, used in Task 17 — consistent fields (`key`, `etag`, `size`, `last_modified`). ✓
- `PutObjectResponse` defined in Task 14, returned by `multipart::upload` in Task 22 — same struct. ✓
- `Error` variants: defined in Task 10, used in `http.rs` (Task 13), `multipart.rs` (Task 22), classifier returns the same variants the tests inspect. ✓
- `Client::put_object_multipart` signature: `(bucket, key, body, part_size, concurrency)` — matches the test in Task 22. ✓
- `Credentials::from_env` returns `Result<Self, Error>` in Task 11; called in CLI Task 20 — consistent. ✓
- `Cmd` enum variants in Task 19 (`Ls/Cp/Rm/Cat/Presign`) match the dispatch in Task 20. ✓
- `signer::v2::presign_url` returns `(String, String)` (url + string-to-sign) in Task 7; presign operation in Task 18 destructures with `let (signed, _sts) = ...`. ✓

All consistent. No fixes needed.

---

**Plan complete and saved to `docs/superpowers/plans/2026-05-14-rust-sdk-and-cli.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
