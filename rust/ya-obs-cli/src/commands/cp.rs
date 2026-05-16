use std::path::{Path, PathBuf};

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
    recursive: bool,
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

    if recursive {
        return match (src_ep, dst_ep) {
            (Endpoint::Local(dir), Endpoint::Remote(u)) => {
                upload_dir(client, &dir, &u, output, quiet).await
            }
            (Endpoint::Remote(u), Endpoint::Local(dir)) => {
                download_prefix(client, &u, &dir, output, quiet).await
            }
            (Endpoint::Stdin, _) | (_, Endpoint::Stdout) => {
                Err(anyhow!("cp --recursive cannot use stdin/stdout"))
            }
            (Endpoint::Remote(_), Endpoint::Remote(_)) => {
                Err(anyhow!("server-side recursive copy not implemented"))
            }
            _ => Err(anyhow!("cp --recursive requires one local and one obs:// side")),
        };
    }

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

/// Walk `dir` and PUT every regular file under `dst_uri.key` as the dest prefix.
/// Symlinks are followed via std::fs::metadata; circular trees will diverge —
/// avoid them. Sequential uploads; on first error the whole copy aborts.
async fn upload_dir(
    client: &Client,
    dir: &str,
    dst_uri: &ObsUri,
    output: OutputFormat,
    quiet: bool,
) -> Result<()> {
    let root = Path::new(dir);
    if !root.is_dir() {
        return Err(anyhow!("not a directory: {dir}"));
    }
    let dst_prefix = normalize_remote_prefix(&dst_uri.key);
    let mut files: Vec<PathBuf> = Vec::new();
    walk_files(root, &mut files)?;

    let mut total_bytes: usize = 0;
    for path in &files {
        let rel = path
            .strip_prefix(root)
            .map_err(|_| anyhow!("strip_prefix failed for {}", path.display()))?;
        // Always use '/' on the wire even if running on Windows-style paths.
        let key_rel = rel
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF-8 path: {}", path.display()))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        let key = format!("{dst_prefix}{key_rel}");

        let target = ObsUri {
            bucket: dst_uri.bucket.clone(),
            key,
        };
        let n = upload_file(client, path.to_str().unwrap(), &target, quiet).await?;
        total_bytes += n;
        emit_summary(
            output,
            path.to_str().unwrap(),
            &format!("obs://{}/{}", target.bucket, target.key),
            n,
            n >= MULTIPART_THRESHOLD,
        );
    }
    if matches!(output, OutputFormat::Json) {
        let line = serde_json::json!({
            "ok": true,
            "done": true,
            "files": files.len(),
            "bytes": total_bytes,
        });
        println!("{line}");
    } else if !quiet {
        eprintln!("done: {} file(s), {} bytes", files.len(), total_bytes);
    }
    Ok(())
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        let meta = std::fs::metadata(&p)?;
        if meta.is_file() {
            out.push(p);
        } else if meta.is_dir() {
            walk_files(&p, out)?;
        }
        // Skip symlinks-to-nothing, fifos, sockets, etc.
    }
    Ok(())
}

/// List every object under `src_uri.key` (recursive, no delimiter) and write
/// each one to a file under `local_dir`, recreating subdirectories as needed.
async fn download_prefix(
    client: &Client,
    src_uri: &ObsUri,
    local_dir: &str,
    output: OutputFormat,
    quiet: bool,
) -> Result<()> {
    let root = Path::new(local_dir);
    std::fs::create_dir_all(root)?;

    let src_prefix = normalize_remote_prefix(&src_uri.key);
    let list_prefix = if src_prefix.is_empty() {
        None
    } else {
        Some(src_prefix.as_str())
    };
    let objects = client.list_objects(&src_uri.bucket, list_prefix).await?;

    let mut total_bytes: usize = 0;
    for o in &objects {
        let rel = o
            .key
            .strip_prefix(&src_prefix)
            .unwrap_or(o.key.as_str())
            // Defensive: if the key starts with '/' after stripping, drop it.
            .trim_start_matches('/');
        if rel.is_empty() {
            // Object key == prefix (a placeholder "directory" object). Skip.
            continue;
        }
        let dest = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let src_one = ObsUri {
            bucket: src_uri.bucket.clone(),
            key: o.key.clone(),
        };
        let n = download_file(client, &src_one, dest.to_str().unwrap(), quiet).await?;
        total_bytes += n;
        emit_summary(
            output,
            &format!("obs://{}/{}", src_one.bucket, src_one.key),
            dest.to_str().unwrap(),
            n,
            false,
        );
    }
    if matches!(output, OutputFormat::Json) {
        let line = serde_json::json!({
            "ok": true,
            "done": true,
            "files": objects.len(),
            "bytes": total_bytes,
        });
        println!("{line}");
    } else if !quiet {
        eprintln!("done: {} file(s), {} bytes", objects.len(), total_bytes);
    }
    Ok(())
}

/// Append a single '/' to a non-empty prefix that doesn't already end in one,
/// so we can join relative paths without producing things like "fookey.txt"
/// from prefix "foo" + key "key.txt".
fn normalize_remote_prefix(key: &str) -> String {
    if key.is_empty() || key.ends_with('/') {
        key.to_string()
    } else {
        format!("{key}/")
    }
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
