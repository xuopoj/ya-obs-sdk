use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ya_obs::Client;

use crate::args::OutputFormat;
use crate::obs_uri::ObsUri;
use crate::progress::{bytes_bar, hidden_bar};

const MULTIPART_THRESHOLD: usize = 100 * 1024 * 1024;

enum Endpoint {
    Local(String),
    Remote(ObsUri),
    Stdin,
    Stdout,
}

fn classify_remote_or_local(s: &str) -> Endpoint {
    if let Ok(u) = s.parse::<ObsUri>() {
        Endpoint::Remote(u)
    } else {
        Endpoint::Local(s.to_string())
    }
}

pub async fn run(
    client: &Client,
    src: &str,
    dst: &str,
    output: OutputFormat,
    quiet: bool,
) -> Result<()> {
    // `-` means stdin when used as src, stdout when used as dst.
    let src_ep = if src == "-" {
        Endpoint::Stdin
    } else {
        classify_remote_or_local(src)
    };
    let dst_ep = if dst == "-" {
        Endpoint::Stdout
    } else {
        classify_remote_or_local(dst)
    };

    match (src_ep, dst_ep) {
        (Endpoint::Local(path), Endpoint::Remote(u)) => {
            let n = upload_file(client, &path, &u, quiet).await?;
            emit_summary(output, src, dst, n, n >= MULTIPART_THRESHOLD);
            Ok(())
        }
        (Endpoint::Stdin, Endpoint::Remote(u)) => {
            let n = upload_stdin(client, &u).await?;
            emit_summary(output, src, dst, n, n >= MULTIPART_THRESHOLD);
            Ok(())
        }
        (Endpoint::Remote(u), Endpoint::Local(path)) => {
            let n = download_file(client, &u, &path, quiet).await?;
            emit_summary(output, src, dst, n, false);
            Ok(())
        }
        (Endpoint::Remote(u), Endpoint::Stdout) => {
            let n = download_stdout(client, &u).await?;
            emit_summary(output, src, dst, n, false);
            Ok(())
        }
        (Endpoint::Remote(_), Endpoint::Remote(_)) => {
            Err(anyhow!("server-side copy not yet implemented"))
        }
        // Any other combination (local-only, stdin-to-local, etc.) is invalid:
        // cp requires exactly one obs:// side.
        _ => Err(anyhow!("cp requires exactly one obs:// side (use `-` for stdin/stdout)")),
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

async fn upload_file(client: &Client, path: &str, dst: &ObsUri, quiet: bool) -> Result<usize> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let bar = if quiet { hidden_bar() } else { bytes_bar(buf.len() as u64) };
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

async fn download_file(client: &Client, src: &ObsUri, path: &str, quiet: bool) -> Result<usize> {
    let resp = client.get_object(&src.bucket, &src.key).await?;
    let total = resp.content_length.unwrap_or(0);
    let bar = if quiet { hidden_bar() } else { bytes_bar(total) };
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

/// Read all of stdin, then PUT. The body is buffered because the OBS PUT
/// path signs the body hash upfront; true streaming would need multipart
/// support in the library. Mind the memory cost for very large pipes.
async fn upload_stdin(client: &Client, dst: &ObsUri) -> Result<usize> {
    let mut buf = Vec::new();
    tokio::io::stdin().read_to_end(&mut buf).await?;
    let len = buf.len();
    client
        .put_object(&dst.bucket, &dst.key, Bytes::from(buf))
        .await?;
    Ok(len)
}

/// Stream the object body to stdout. No progress bar — stdout is the data
/// channel, and writing one to a terminal would interleave with the bytes.
async fn download_stdout(client: &Client, src: &ObsUri) -> Result<usize> {
    let resp = client.get_object(&src.bucket, &src.key).await?;
    let mut out = tokio::io::stdout();
    let mut stream = Box::pin(resp.body.into_stream());
    let mut written = 0usize;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        out.write_all(&chunk).await?;
        written += chunk.len();
    }
    out.flush().await?;
    Ok(written)
}
