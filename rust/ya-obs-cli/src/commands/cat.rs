use anyhow::Result;
use tokio::io::AsyncWriteExt;
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;

pub async fn run(client: &Client, uri: &str, output: OutputFormat) -> Result<()> {
    let parsed: ObsUri = uri.parse()?;
    let resp = client.get_object(&parsed.bucket, &parsed.key).await?;
    let bytes = resp.body.read_to_end().await?;
    let len = bytes.len();
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&bytes).await?;
    stdout.flush().await?;

    // JSON summary goes to STDERR so it never corrupts the body on stdout.
    if matches!(output, OutputFormat::Json) {
        let line = serde_json::json!({
            "ok": true,
            "bucket": parsed.bucket,
            "key": parsed.key,
            "bytes": len,
        });
        eprintln!("{line}");
    }
    Ok(())
}
