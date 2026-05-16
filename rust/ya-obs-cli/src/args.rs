use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "ya-obs",
    version,
    about = "Huawei Cloud OBS CLI",
    long_about = "Huawei Cloud OBS CLI.\n\
        \n\
        EXIT CODES\n\
        \n  \
        0   success\n  \
        1   generic / transport / unspecified\n  \
        2   usage error (bad CLI args)\n  \
        3   config error (missing creds/region, bad signing version)\n  \
        4   not found (NoSuchKey, NoSuchBucket)\n  \
        5   access denied (AccessDenied, 401, 403)\n  \
        6   server error (5xx)\n\
        \n\
        IDEMPOTENCY & RETRIES\n\
        \n  \
        The library retries 408/429/5xx with exponential backoff. The CLI\n  \
        itself does not retry beyond that. PUT and DELETE are safe to retry;\n  \
        OBS DELETE on a missing key is idempotent (returns success).\n\
        \n\
        JSON OUTPUT\n\
        \n  \
        Pass -o json (global) for stable machine-readable output:\n  \
          ls       NDJSON, one object per line\n  \
          stat     single object {bucket,key,size,etag,content_type,...}\n  \
          presign  {url}\n  \
          cp       {ok,src,dst,bytes,multipart}\n  \
          rm       {ok,bucket,key[,dry_run]}\n  \
          cat      body on stdout; {ok,bucket,key,bytes} on stderr\n  \
          errors   {error:{code,message,status}} on stderr"
)]
pub struct Cli {
    /// Named profile from the config file. Defaults to [default].
    #[arg(long, env = "YA_OBS_PROFILE")]
    pub profile: Option<String>,

    /// Path to config file. Defaults to $XDG_CONFIG_HOME/ya-obs/config.toml
    /// (typically ~/.config/ya-obs/config.toml).
    #[arg(long, env = "YA_OBS_CONFIG")]
    pub config: Option<String>,

    /// Region (e.g. cn-north-4). Required for V4 signing even when --endpoint is set.
    #[arg(long, env = "YA_OBS_REGION")]
    pub region: Option<String>,

    /// Custom endpoint URL (overrides region-derived URL, but does not replace --region for signing).
    #[arg(long, env = "YA_OBS_ENDPOINT")]
    pub endpoint: Option<String>,

    /// Access key (defaults to $HUAWEICLOUD_SDK_AK).
    #[arg(long, env = "HUAWEICLOUD_SDK_AK", hide_env_values = true)]
    pub access_key: Option<String>,

    /// Secret key (defaults to $HUAWEICLOUD_SDK_SK).
    #[arg(long, env = "HUAWEICLOUD_SDK_SK", hide_env_values = true)]
    pub secret_key: Option<String>,

    /// Signing version: "v4" (default) or "v2".
    #[arg(long, value_enum)]
    pub signing_version: Option<SignVer>,

    /// Disable TLS certificate verification. Use only with private CAs you trust.
    #[arg(long)]
    pub insecure: bool,

    /// Output format. `text` (default) is human-readable; `json` emits a
    /// stable machine-readable shape per subcommand (NDJSON for `ls`,
    /// single objects for the others).
    #[arg(long, short = 'o', value_enum, default_value_t = OutputFormat::Text, global = true)]
    pub output: OutputFormat,

    /// Suppress informational stderr (TLS-disabled warning, progress bars).
    /// Errors still print. Set `RUST_LOG=warn` to surface suppressed warnings.
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SignVer {
    V4,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List buckets (no URI) or list objects under an obs:// URI.
    #[command(long_about = "List buckets when invoked with no URI; list \
        objects under a prefix when given obs://bucket/[prefix].\n\
        \n\
        By default `ls` is delimited — like `aws s3 ls`, only entries at the\n\
        current level are shown, with sub-prefixes marked `PRE foo/`. Use\n\
        `-r` to flatten and recurse into every object under the prefix.\n\
        \n\
        Examples:\n  \
          ya-obs ls                                # list all buckets\n  \
          ya-obs ls obs://bucket                   # top-level entries in bucket\n  \
          ya-obs ls obs://bucket/foo/              # entries under foo/\n  \
          ya-obs ls -r obs://bucket                # every object, recursive")]
    Ls {
        uri: Option<String>,
        /// Recurse into every object (no delimiter). Equivalent to listing
        /// without `delimiter=/`.
        #[arg(long, short = 'r')]
        recursive: bool,
    },
    /// Copy between local and obs://.
    #[command(long_about = "Copy between local paths and obs:// URIs.\n\
        \n\
        One side must be obs://. Use \"-\" for stdin (as src) or stdout\n\
        (as dst). Stdin uploads buffer the whole body in memory before\n\
        signing; for large pipes prefer staging to a file first.\n\
        \n\
        Multipart kicks in automatically above 100 MB for uploads.")]
    Cp { src: String, dst: String },
    /// Delete an object.
    #[command(long_about = "Delete an object.\n\
        \n\
        OBS DELETE is idempotent: removing a missing key returns success\n\
        (exit 0), not NoSuchKey. Use `stat` first if you need to confirm\n\
        existence before deletion.\n\
        \n\
        Use --dry-run to preview without sending the DELETE.")]
    Rm {
        uri: String,
        /// Skip the DELETE request; print what would be deleted and exit 0.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print object body to stdout.
    Cat { uri: String },
    /// Fetch object metadata (HEAD). Exits 4 if the object doesn't exist.
    #[command(long_about = "Fetch object metadata via a HEAD request.\n\
        \n\
        Prints size, etag, content_type, and request_id. Exits 4 if the\n\
        object doesn't exist — agents can use this as a cheap existence\n\
        check without listing a prefix.")]
    Stat { uri: String },
    /// Generate a presigned GET URL.
    Presign {
        uri: String,
        /// Expiry in seconds.
        #[arg(long, default_value_t = 3600)]
        expires: u64,
    },
    /// Create a starter config file at the XDG location (~/.config/ya-obs/config.toml).
    Init {
        /// Custom path (overrides --config and the default XDG location).
        #[arg(long)]
        path: Option<String>,
        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,
    },
    /// Print a shell completion script (bash | zsh).
    #[command(long_about = "Print a tab-completion script for the given shell. \
        Static (subcommands, flags, enum values) plus dynamic obs:// path \
        completion that consults the live cluster.\n\
        \n\
        Install for bash:\n  \
          ya-obs completion bash > ~/.local/share/bash-completion/completions/ya-obs\n\
        \n\
        Install for zsh (ensure the dir is in your fpath):\n  \
          ya-obs completion zsh > ~/.zfunc/_ya-obs")]
    Completion { shell: CompletionShell },
    /// Internal helper used by completion scripts to expand obs:// paths.
    /// Reads one partial URI, writes one candidate per line on stdout.
    #[command(name = "__complete-obs", hide = true)]
    CompleteObs { input: String },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
}
