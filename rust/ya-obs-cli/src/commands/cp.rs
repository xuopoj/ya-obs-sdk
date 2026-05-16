use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;
use crate::progress::bytes_bar;

const MULTIPART_THRESHOLD: usize = 100 * 1024 * 1024;

enum Endpoint {
    Local(String),
    Remote(ObsUri),
}

fn classify(s: &str) -> Endpoint {
    if let Ok(u) = s.parse::<ObsUri>() {
        Endpoint::Remote(u)
    } else {
        Endpoint::Local(s.to_string())
    }
}

pub async fn run(client: &Client, src: &str, dst: &str, output: OutputFormat) -> Result<()> {
    match (classify(src), classify(dst)) {
        (Endpoint::Local(path), Endpoint::Remote(u)) => {
            let bytes_written = upload(client, &path, &u).await?;
            emit_summary(output, src, dst, bytes_written, bytes_written >= MULTIPART_THRESHOLD);
            Ok(())
        }
        (Endpoint::Remote(u), Endpoint::Local(path)) => {
            let bytes_written = download(client, &u, &path).await?;
            emit_summary(output, src, dst, bytes_written, false);
            Ok(())
        }
        (Endpoint::Local(_), Endpoint::Local(_)) => {
            Err(anyhow!("at least one side of cp must be obs://"))
        }
        (Endpoint::Remote(_), Endpoint::Remote(_)) => {
            Err(anyhow!("server-side copy not yet implemented"))
        }
    }
}

fn emit_summary(output: OutputFormat, src: &str, dst: &str, bytes: usize, multipart: bool) {
    if matches!(output, OutputFormat::Json) {
        let line = serde_json::json!({
            "ok": true,
            "src": src,
            "dst": dst,
            "bytes": bytes,
            "multipart": multipart,
        });
        println!("{line}");
    }
}

async fn upload(client: &Client, path: &str, dst: &ObsUri) -> Result<usize> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let bar = bytes_bar(buf.len() as u64);
    bar.set_message(format!(
        "uploading {} -> obs://{}/{}",
        path, dst.bucket, dst.key
    ));
    let len = buf.len();
    client
        .put_object(&dst.bucket, &dst.key, Bytes::from(buf))
        .await?;
    bar.inc(len as u64);
    bar.finish_with_message("done");
    Ok(len)
}

async fn download(client: &Client, src: &ObsUri, path: &str) -> Result<usize> {
    let resp = client.get_object(&src.bucket, &src.key).await?;
    let total = resp.content_length.unwrap_or(0);
    let bar = bytes_bar(total);
    bar.set_message(format!(
        "downloading obs://{}/{} -> {}",
        src.bucket, src.key, path
    ));

    let mut out = File::create(path).await?;
    let mut stream = Box::pin(resp.body.into_stream());
    let mut written = 0usize;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        out.write_all(&chunk).await?;
        bar.inc(chunk.len() as u64);
        written += chunk.len();
    }
    out.flush().await?;
    bar.finish_with_message("done");
    Ok(written)
}
