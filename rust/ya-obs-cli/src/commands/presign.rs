use anyhow::Result;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub fn run(client: &Client, uri: &str, expires: u64) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let url = client.presign_get_object(&parsed.bucket, &parsed.key, expires)?;
    println!("{url}");
    Ok(())
}
