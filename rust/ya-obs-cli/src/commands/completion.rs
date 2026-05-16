use anyhow::Result;
use ya_obs::Client;

use crate::args::CompletionShell;

const BASH: &str = include_str!("../../completions/ya-obs.bash");
const ZSH: &str = include_str!("../../completions/_ya-obs.zsh");

pub fn print_script(shell: CompletionShell) -> Result<()> {
    let script = match shell {
        CompletionShell::Bash => BASH,
        CompletionShell::Zsh => ZSH,
    };
    print!("{script}");
    Ok(())
}

/// Expand a partial obs:// URI into completion candidates, one per line.
///
/// Conventions used by the shell scripts:
///   "obs://"             -> list buckets, emit "obs://<name>/"
///   "obs://bu"           -> list buckets starting with "bu", emit "obs://<name>/"
///   "obs://bucket/"      -> list at root of bucket; subdirs as "obs://bucket/sub/",
///                           objects as "obs://bucket/key"
///   "obs://bucket/foo/"  -> list under foo/ similarly
///   "obs://bucket/foo/p" -> list under foo/ where key starts with "foo/p"
pub async fn complete_obs(client: &Client, input: &str) -> Result<()> {
    let rest = match input.strip_prefix("obs://") {
        Some(r) => r,
        None => return Ok(()), // not an obs:// URI; emit nothing
    };

    match rest.split_once('/') {
        // No slash yet: still typing the bucket name.
        None => {
            let buckets = client.list_buckets().await?;
            for b in buckets {
                if b.name.starts_with(rest) {
                    println!("obs://{}/", b.name);
                }
            }
        }
        // A slash is present: list under (bucket, prefix-up-to-last-slash).
        Some((bucket, after_slash)) => {
            // Split after_slash into (dir_prefix, partial_leaf).
            // For "foo/p", dir_prefix="foo/", leaf="p".
            // For "foo/", dir_prefix="foo/", leaf="".
            // For "", dir_prefix="", leaf="".
            let (dir_prefix, _leaf) = match after_slash.rfind('/') {
                Some(i) => (&after_slash[..=i], &after_slash[i + 1..]),
                None => ("", after_slash),
            };

            let list_prefix = if after_slash.is_empty() {
                None
            } else {
                Some(after_slash)
            };
            let result = client
                .list_objects_delimited(bucket, list_prefix, Some("/"))
                .await?;

            // CommonPrefixes (subdirectories)
            for cp in result.common_prefixes {
                // cp is the full prefix relative to bucket root (e.g. "foo/sub/")
                println!("obs://{bucket}/{cp}");
            }
            // Objects at this level
            for o in result.objects {
                println!("obs://{bucket}/{}", o.key);
            }

            // Avoid unused warning if the user wants to also filter by partial leaf
            let _ = dir_prefix;
        }
    }
    Ok(())
}
