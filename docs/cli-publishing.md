# Publishing artifacts with ya-obs

These capabilities target the next CLI release. The currently published 0.1.2
binary does not have the new flags. Live acceptance is still required before
this version is approved for the aishipbox.site migration.

## Single-file uploads

```sh
export YA_OBS_REGION=cn-southwest-2
# Supply HUAWEICLOUD_SDK_AK and HUAWEICLOUD_SDK_SK through Actions Secrets.
sha256=$(sha256sum skill.zip | cut -d ' ' -f 1)
ya-obs -o json cp skill.zip "obs://aishipbox/skills/example/1.0.0/$sha256/skill.zip" \
  --no-clobber --acl public-read \
  --content-type application/zip \
  --cache-control 'public, max-age=31536000, immutable' \
  --content-disposition 'attachment; filename="skill.zip"' \
  --metadata "sha256=$sha256"
```

`--no-clobber` sends `x-obs-forbid-overwrite: true` on every PUT attempt.
It first reads bucket versioning and bucket metadata. Enabled or suspended
versioning, parallel filesystems, unknown state, or failed preflight prevent
upload. Permissions must include GetBucketVersioning and HeadBucket as well as
PutObject. The implementation relies on OBS's documented semantics; arbitrary
S3-compatible endpoints may not honor this header. Bucket administrators must
not enable versioning between preflight and upload; the two operations are not
atomic. For long-lived publishing buckets, restrict versioning configuration
changes separately from the publishing credentials.

All new upload flags apply only to single local files smaller than 100 MiB.
Recursive uploads, stdin uploads, downloads, server-side copies, and multipart
uploads reject these options with exit 8. Ordinary `cp` retains its existing
behavior, including automatic multipart upload at 100 MiB. No option defaults
to public access. Metadata keys use lowercase ASCII letters, digits, hyphens,
or underscores; repeated keys and invalid header values are rejected. There
is no general header override for authentication, Host, or signing fields.

A conflict is never treated as success, including after a retry. The caller
must download and compare bytes before skipping an existing object. `stat`
JSON adds `cache_control`, `content_disposition`, and `metadata`; ETag is not
an archive SHA-256. TLS verification remains enabled by default.

## Repair an existing private object's ACL

First authenticate, download, and verify the bytes and SHA-256. Then:

```sh
ya-obs -o json acl set obs://aishipbox/skills/example/1.0.0/HASH/skill.zip --acl public-read
ya-obs -o json stat obs://aishipbox/skills/example/1.0.0/HASH/skill.zip
ya-obs -o json bucket-versioning obs://aishipbox
```

ACL changes target a nonempty object key, never the bucket policy. They require
PutObjectAcl permission. An ACL success response does not prove anonymous
access: perform an anonymous HTTPS GET and compare size and SHA-256 before
adding an object to the public mirror index. Bucket policies may still block
public reads.

## Automation errors

Errors retain `{ "error": { "code": "...", "message": "...", "status": 0 } }`
on stderr in JSON mode. Existing exit codes remain unchanged:

| Exit | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Transport or other error |
| 2 | Invalid CLI syntax |
| 3 | Configuration or invalid option value |
| 4 | Object or bucket not found |
| 5 | Access denied / HTTP 401 or 403 |
| 6 | HTTP 5xx after retries |
| 7 | HTTP 409 or 412 conflict; inspect service code and verify existing bytes |
| 8 | Unsupported protection condition or upload combination |

The original OBS error code is preserved when an XML error is available.
Empty HTTP 409 bodies produce code `Conflict`; unsupported local checks produce
`Unsupported`. No confirmation prompt or stdin read is used in these commands.

## Pinned Actions installation

After live acceptance and publishing, set repository variables `YA_OBS_VERSION`
(e.g. `0.1.3`) and `YA_OBS_LINUX_X64_SHA256` to the reviewed version and exact
64-character archive digest. Never set them from an unpinned `latest` lookup.
The expected digest is independently pinned; downloading a checksum alongside
an archive alone does not establish that pin.

```yaml
- name: Install pinned ya-obs
  env:
    VERSION: ${{ vars.YA_OBS_VERSION }}
    EXPECTED_SHA256: ${{ vars.YA_OBS_LINUX_X64_SHA256 }}
  shell: bash
  run: |
    set -euo pipefail
    [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
    [[ "$EXPECTED_SHA256" =~ ^[0-9a-f]{64}$ ]]
    asset=ya-obs-x86_64-unknown-linux-gnu.tar.gz
    base="https://github.com/xuopoj/ya-obs-sdk/releases/download/ya-obs-cli-v$VERSION"
    cd "$RUNNER_TEMP"
    curl --fail --location --proto '=https' --tlsv1.2 --output "$asset" "$base/$asset"
    curl --fail --location --proto '=https' --tlsv1.2 --output "$asset.sha256" "$base/$asset.sha256"
    printf '%s  %s\n' "$EXPECTED_SHA256" "$asset" | sha256sum --check -
    sha256sum --check "$asset.sha256"
    tar -xzf "$asset" ya-obs
    mkdir -p "$HOME/.local/bin"
    install -m 755 ya-obs "$HOME/.local/bin/ya-obs"
    echo "$HOME/.local/bin" >> "$GITHUB_PATH"
    "$HOME/.local/bin/ya-obs" --version
```

## Live acceptance and release order

Local planning only (does not read credentials or upload):

```sh
python3 scripts/accept_publishing.py --bucket DEDICATED-TEST-BUCKET
```

Configure the GitHub environment `obs-publishing-test` with `OBS_TEST_ACCESS_KEY`
and `OBS_TEST_SECRET_KEY`. Dispatch `publishing-acceptance.yml` on the candidate
commit with a dedicated, unversioned test bucket and its region. The test uses
only a fresh `_ya-obs-acceptance/UUID/` prefix, tests public GET, metadata,
conflict preservation, concurrent creates and private-to-public ACL repair,
and removes objects whose creation was confirmed. Failed ambiguous uploads
may leave test objects under the logged prefix for manual inspection. Bucket
settings and policies are never changed. The test bucket must permit object
ACLs to control access; a bucket policy granting anonymous reads would defeat
the private-object check. Permission and unsupported-state errors are tested
with local mock servers rather than mutating bucket settings.

Stamp the version and changelog, commit, run live acceptance on that exact
commit, then tag it `ya-obs-cli-vVERSION`. Dispatch `release.yml` with that tag. It checks out the
exact tag, runs formatting/Clippy/tests, verifies five archives and checksums,
and smoke-tests the Linux binary. It creates or resumes a draft, uploads all
assets and notes, and only publishes when `draft=false` and a successful
`publishing-acceptance.yml` run exists for the exact release commit. A published release
is immutable to this workflow; it refuses to replace its assets. A failed
build or partial upload leaves the existing latest release unchanged.

The aishipbox.site migration remains separate: pin the accepted release and
digest, preserve byte verification and mirror records, then rerun existing
packages to verify they are skipped without overwriting.

References: [OBS overwrite restrictions](https://support.huaweicloud.com/perms-cfg-obs/obs_40_0041.html),
[bucket versioning](https://support.huaweicloud.com/api-obs/obs_04_0038.html),
[bucket metadata](https://support.huaweicloud.com/api-obs/obs_04_0023.html).
