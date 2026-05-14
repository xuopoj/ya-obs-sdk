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
- Conformance: 100% auth vectors (4 V4 + 4 V2), 100% xml vectors (4), 100%
  error vectors (3). Multipart scenario test against wiremock validates the
  initiate/part/complete sequence. Total: 39 tests passing across the workspace.
