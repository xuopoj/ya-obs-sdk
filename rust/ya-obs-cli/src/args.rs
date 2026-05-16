use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ya-obs", version, about = "Huawei Cloud OBS CLI")]
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
    /// List objects under an obs:// URI.
    Ls { uri: String },
    /// Copy between local and obs://.
    Cp { src: String, dst: String },
    /// Delete an object.
    Rm { uri: String },
    /// Print object body to stdout.
    Cat { uri: String },
    /// Fetch object metadata (HEAD). Exits 4 if the object doesn't exist.
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
}
