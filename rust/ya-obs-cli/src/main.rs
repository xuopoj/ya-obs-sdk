use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use ya_obs::{AddressingStyle, Client, ClientConfig, Credentials, SigningVersion};
use ya_obs_cli::args::{Cli, Cmd, SignVer};
use ya_obs_cli::commands;
use ya_obs_cli::config::{default_config_path, load, pick_first, Profile};

fn parse_signing_version(s: &str) -> Result<SigningVersion> {
    match s.to_ascii_lowercase().as_str() {
        "v4" => Ok(SigningVersion::V4),
        "v2" => Ok(SigningVersion::V2),
        other => Err(anyhow!(
            "invalid signing_version {other:?}; expected v4 or v2"
        )),
    }
}

fn build_client(cli: &Cli) -> Result<Client> {
    // Load the config file (if any). The Cli's profile/config fields come from
    // CLI flags, env vars, or are None.
    let config_path: Option<PathBuf> = cli
        .config
        .as_ref()
        .map(PathBuf::from)
        .or_else(default_config_path);

    let loaded = match &config_path {
        Some(p) => load(p, cli.profile.as_deref()).map_err(|e| anyhow!(e))?,
        None => Default::default(),
    };

    for w in &loaded.warnings {
        eprintln!("ya-obs: {w}");
    }
    let file: Profile = loaded.profile;

    // Effective values: CLI flag > env var > config file.
    // (Clap already merged CLI + env into the Cli struct, so for those fields
    // we just need to layer the config file underneath.)
    let region = pick_first([cli.region.clone(), None, file.region.clone()]);
    let endpoint = pick_first([cli.endpoint.clone(), None, file.endpoint.clone()]);
    let access_key = pick_first([cli.access_key.clone(), None, file.access_key.clone()]);
    let secret_key = pick_first([cli.secret_key.clone(), None, file.secret_key.clone()]);

    let signing_version = match cli.signing_version {
        Some(SignVer::V4) => SigningVersion::V4,
        Some(SignVer::V2) => SigningVersion::V2,
        None => match file.signing_version.as_deref() {
            Some(s) => parse_signing_version(s)?,
            None => SigningVersion::V4,
        },
    };

    let mut cfg = match (&endpoint, &region) {
        (Some(ep), region_opt) => {
            let mut c = ClientConfig::for_endpoint(ep);
            c.region = region_opt.clone();
            c
        }
        (None, Some(r)) => ClientConfig::for_region(r),
        (None, None) => {
            return Err(anyhow!(
                "set --region or --endpoint (or env YA_OBS_REGION / YA_OBS_ENDPOINT, \
                 or a [default] section in {})",
                config_path
                    .as_deref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "~/.config/ya-obs/config.toml".into())
            ))
        }
    };

    if signing_version == SigningVersion::V4 && cfg.region.is_none() {
        return Err(anyhow!(
            "V4 signing requires --region (or YA_OBS_REGION, or `region = ...` in the \
             config file); endpoint alone is not enough. \
             Use --signing-version v2 if your cluster only supports legacy OBS signing."
        ));
    }

    let creds = match (access_key, secret_key) {
        (Some(ak), Some(sk)) => Credentials::new(ak, sk),
        _ => Credentials::from_env()?,
    };
    cfg = cfg.with_credentials(creds);
    cfg = cfg.with_signing_version(signing_version);
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
        Cmd::Cp { src, dst } => commands::cp::run(&client, src, dst).await?,
    }
    Ok(())
}
