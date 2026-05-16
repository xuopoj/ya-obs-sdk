use anyhow::{anyhow, Result};
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str, output: OutputFormat, dry_run: bool) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    if parsed.key.is_empty() {
        return Err(anyhow!(
            "rm requires an object key, got bucket-only URI {uri}"
        ));
    }

    if !dry_run {
        client.delete_object(&parsed.bucket, &parsed.key).await?;
    }

    match output {
        OutputFormat::Text => {
            if dry_run {
                println!("would delete obs://{}/{}", parsed.bucket, parsed.key);
            }
        }
        OutputFormat::Json => {
            let mut obj = serde_json::json!({
                "ok": true,
                "bucket": parsed.bucket,
                "key": parsed.key,
            });
            if dry_run {
                obj.as_object_mut()
                    .unwrap()
                    .insert("dry_run".into(), serde_json::Value::Bool(true));
            }
            println!("{obj}");
        }
    }
    Ok(())
}
