use anyhow::{anyhow, Result};
use clap::Parser;
use ya_obs::{AddressingStyle, Client, ClientConfig, Credentials, SigningVersion};
use ya_obs_cli::args::{Cli, Cmd, SignVer};
use ya_obs_cli::commands;

fn build_client(cli: &Cli) -> Result<Client> {
    let mut cfg = match (&cli.endpoint, &cli.region) {
        (Some(ep), _) => ClientConfig::for_endpoint(ep),
        (None, Some(r)) => ClientConfig::for_region(r),
        (None, None) => {
            return Err(anyhow!(
                "set --region or --endpoint (or env YA_OBS_REGION / YA_OBS_ENDPOINT)"
            ))
        }
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
        Cmd::Cp { src, dst } => commands::cp::run(&client, src, dst).await?,
    }
    Ok(())
}
