# Changelog

All notable changes to the `ya-obs` and `ya-obs-cli` crates.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- `ya-obs init` subcommand scaffolds a starter config at
  `~/.config/ya-obs/config.toml` (or `--path`); refuses to overwrite without
  `--force`; sets file mode to 0600 on Unix.

## [0.1.1] - 2026-05-14

### Added
- `ya-obs-cli` reads `$XDG_CONFIG_HOME/ya-obs/config.toml` (defaulting to
  `~/.config/ya-obs/config.toml`) with `[default]` and `[profiles.<name>]`
  sections. Select a profile via `--profile` / `$YA_OBS_PROFILE`. Precedence:
  CLI flag > env var > config file. World/group-readable config files
  containing credentials surface a permissions warning.

### Changed
- `ya-obs-cli` no longer silently drops `--region` when `--endpoint` is also
  set; both are honored. Endpoint controls the URL host, region controls
  V4 signing scope.
- `--signing-version` defaults remain V4 but the flag is now optional so the
  config file can supply the default.

### Fixed
- Clearer error when V4 signing is selected without a region.
- `clippy::unnecessary_sort_by` lint surfaced by stable rustc 1.95.

## [0.1.0] - 2026-05-14

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
