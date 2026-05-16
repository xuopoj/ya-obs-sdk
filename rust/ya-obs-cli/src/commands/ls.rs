use anyhow::Result;
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(
    client: &Client,
    uri: Option<&str>,
    recursive: bool,
    output: OutputFormat,
) -> Result<()> {
    match uri {
        None => list_buckets(client, output).await,
        Some(u) => list_objects(client, u, recursive, output).await,
    }
}

async fn list_buckets(client: &Client, output: OutputFormat) -> Result<()> {
    let buckets = client.list_buckets().await?;
    match output {
        OutputFormat::Text => {
            for b in buckets {
                println!("{}  {}", b.creation_date, b.name);
            }
        }
        OutputFormat::Json => {
            for b in buckets {
                let line = serde_json::json!({
                    "name": b.name,
                    "creation_date": b.creation_date,
                });
                println!("{line}");
            }
        }
    }
    Ok(())
}

async fn list_objects(
    client: &Client,
    uri: &str,
    recursive: bool,
    output: OutputFormat,
) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let prefix = if parsed.key.is_empty() {
        None
    } else {
        Some(parsed.key.as_str())
    };

    if recursive {
        let objects = client.list_objects(&parsed.bucket, prefix).await?;
        emit_objects(&objects, output);
        return Ok(());
    }

    let result = client
        .list_objects_delimited(&parsed.bucket, prefix, Some("/"))
        .await?;

    match output {
        OutputFormat::Text => {
            // Subdirectories first, then objects — like `aws s3 ls`.
            for p in &result.common_prefixes {
                println!("{:>26}  {}", "PRE", p);
            }
            for o in &result.objects {
                println!("{:>12}  {}  {}", o.size, o.last_modified, o.key);
            }
        }
        OutputFormat::Json => {
            for p in &result.common_prefixes {
                let line = serde_json::json!({ "prefix": p });
                println!("{line}");
            }
            for o in &result.objects {
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

fn emit_objects(objects: &[ya_obs::models::ListedObject], output: OutputFormat) {
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
}
