use anyhow::Result;
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str, output: OutputFormat) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let prefix = if parsed.key.is_empty() {
        None
    } else {
        Some(parsed.key.as_str())
    };
    let objects = client.list_objects(&parsed.bucket, prefix).await?;
    match output {
        OutputFormat::Text => {
            for o in objects {
                println!("{:>12}  {}  {}", o.size, o.last_modified, o.key);
            }
        }
        OutputFormat::Json => {
            for o in objects {
                let line = serde_json::json!({
                    "key": o.key,
                    "size": o.size,
                    "etag": o.etag,
                    "last_modified": o.last_modified,
                });
                println!("{line}");
            }
        }
    }
    Ok(())
}
