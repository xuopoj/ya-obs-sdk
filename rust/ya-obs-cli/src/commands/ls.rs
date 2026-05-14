use anyhow::Result;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let prefix = if parsed.key.is_empty() { None } else { Some(parsed.key.as_str()) };
    let objects = client.list_objects(&parsed.bucket, prefix).await?;
    for o in objects {
        println!("{:>12}  {}  {}", o.size, o.last_modified, o.key);
    }
    Ok(())
}
