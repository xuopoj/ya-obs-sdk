use anyhow::{anyhow, Result};
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str, output: OutputFormat) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    if parsed.key.is_empty() {
        return Err(anyhow!(
            "rm requires an object key, got bucket-only URI {uri}"
        ));
    }
    client.delete_object(&parsed.bucket, &parsed.key).await?;

    if matches!(output, OutputFormat::Json) {
        let line = serde_json::json!({
            "ok": true,
            "bucket": parsed.bucket,
            "key": parsed.key,
        });
        println!("{line}");
    }
    Ok(())
}
