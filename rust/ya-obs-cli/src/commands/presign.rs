use anyhow::Result;
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub fn run(client: &Client, uri: &str, expires: u64, output: OutputFormat) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let url = client.presign_get_object(&parsed.bucket, &parsed.key, expires)?;
    match output {
        OutputFormat::Text => println!("{url}"),
        OutputFormat::Json => {
            let line = serde_json::json!({ "url": url });
            println!("{line}");
        }
    }
    Ok(())
}
