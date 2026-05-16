use anyhow::Result;
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str, output: OutputFormat) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let meta = client.head_object(&parsed.bucket, &parsed.key).await?;

    match output {
        OutputFormat::Text => {
            let size = meta
                .content_length
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_string());
            let ct = meta.content_type.as_deref().unwrap_or("-");
            let rid = meta.request_id.as_deref().unwrap_or("-");
            println!("bucket:        {}", parsed.bucket);
            println!("key:           {}", parsed.key);
            println!("size:          {}", size);
            println!("etag:          {}", meta.etag);
            println!("content_type:  {}", ct);
            println!("request_id:    {}", rid);
        }
        OutputFormat::Json => {
            let line = serde_json::json!({
                "bucket": parsed.bucket,
                "key": parsed.key,
                "size": meta.content_length,
                "etag": meta.etag,
                "content_type": meta.content_type,
                "request_id": meta.request_id,
            });
            println!("{line}");
        }
    }
    Ok(())
}
