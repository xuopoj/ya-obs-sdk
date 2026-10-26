# Changelog

All notable changes to the `ya-obs` and `ya-obs-cli` crates.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Publishing options for single local-file uploads below 100 MiB: `--no-clobber`,
  `--acl private|public-read`, `--content-type`, `--cache-control`,
  `--content-disposition`, and repeatable `--metadata key=value`.
- `acl set` changes one object's ACL without rewriting its contents;
  `bucket-versioning` reads the bucket's versioning state.
- `stat` includes cache control, content disposition, and user metadata.
- Exit codes 7 (HTTP 409/412 conflict) and 8 (unsupported protection/options).
- An Actions-only isolated publishing acceptance workflow and pinned installation guide.

### Fixed
- OBS V2 signs Content-Type and ACL/versioning/multipart subresources.
- Signed HTTP requests reject redirects instead of treating them as successful uploads.
- Release builds use the requested tag and validate tests, all checksums and the
  Linux binary before completing a draft release and optionally publishing it.


## [0.1.2] - 2026-10-23

### Added
- `ya-obs init` subcommand scaffolds a starter config at
  `~/.config/ya-obs/config.toml` (or `--path`); refuses to overwrite without
  `--force`; sets file mode to 0600 on Unix.
- `--insecure` CLI flag (and `insecure = true` config key) disables TLS
  certificate verification for endpoints with private CAs. Prints a stderr
  warning when active. Library: `ClientConfig::with_tls_verify_disabled(true)`.
- `Client::list_buckets()` — GET `/` returning `Vec<ListedBucket>`. CLI `ya-obs ls`
  with no URI now lists all buckets in the account; `ya-obs ls obs://bucket`
  continues to list objects. New XML test vector at
  `test-vectors/xml/list_buckets_response.json`.

### Added
- JSON output, typed exit codes, `stat`, `--quiet`, `rm --dry-run`, and stdin/stdout copy support.
- Recursive directory copy, directory-style listing with delimiters, and shell completion for Bash and Zsh.

### Fixed
- OBS V2 signing for custom endpoints, dotted bucket names, and service-root requests.
- Default config path consistently uses `~/.config/ya-obs/config.toml`, including on macOS.
- SigV4 canonical URI encoding for non-ASCII keys and query encoding for slashes.
- Empty error bodies are classified by HTTP status code.

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
