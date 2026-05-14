use anyhow::Result;
use tokio::io::AsyncWriteExt;
use ya_obs::Client;

use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let resp = client.get_object(&parsed.bucket, &parsed.key).await?;
    let bytes = resp.body.read_to_end().await?;
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&bytes).await?;
    stdout.flush().await?;
    Ok(())
}
