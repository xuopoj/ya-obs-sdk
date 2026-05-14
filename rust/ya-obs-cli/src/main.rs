use anyhow::Result;
use clap::Parser;
use ya_obs_cli::args::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    println!("ya-obs cli — subcommands not implemented yet in this task");
    Ok(())
}
