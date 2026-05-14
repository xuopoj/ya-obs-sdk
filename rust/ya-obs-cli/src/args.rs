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
