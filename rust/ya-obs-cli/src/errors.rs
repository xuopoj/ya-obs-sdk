use thiserror::Error;

use crate::args::OutputFormat;

/// Wrapper for user-facing config problems (missing region/creds, bad
/// signing_version string, etc). Surfaces as exit code 3 at the main boundary.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct ConfigError(pub String);

/// Map an anyhow error chain to a process exit code.
///
/// 0 = success (not called here)
/// 1 = generic / transport / unspecified
/// 2 = usage error (clap handles this directly)
/// 3 = configuration error (missing creds, bad signing version, etc.)
/// 4 = NoSuchKey / NoSuchBucket
/// 5 = AccessDenied
/// 6 = server error (5xx)
pub fn classify(err: &anyhow::Error) -> u8 {
    if let Some(obs) = err.downcast_ref::<ya_obs::Error>() {
        return classify_obs(obs);
    }
    if err.downcast_ref::<ConfigError>().is_some() {
        return 3;
    }
    1
}

fn classify_obs(e: &ya_obs::Error) -> u8 {
    use ya_obs::Error as E;
    match e {
        E::NoSuchKey { .. } | E::NoSuchBucket { .. } => 4,
        E::AccessDenied { .. } => 5,
        // 401/403 from other error codes (e.g. InvalidAccessKeyId,
        // SignatureDoesNotMatch) — also auth-failure, treat the same.
        E::Client { status, .. } if *status == 401 || *status == 403 => 5,
        E::Server { status, .. } if *status >= 500 => 6,
        E::Config(_) => 3,
        _ => 1,
    }
}

/// Render an error to a single line on stderr. No anyhow "Caused by:" chain.
///
/// In JSON mode, prints a single-line JSON object on stderr with a stable
/// `error` field: `{"error":{"code":"...","message":"...","status":...}}`.
pub fn display(err: &anyhow::Error, output: OutputFormat) {
    match output {
        OutputFormat::Text => {
            if let Some(obs) = err.downcast_ref::<ya_obs::Error>() {
                eprintln!("error: {obs}");
            } else {
                eprintln!("error: {err}");
            }
        }
        OutputFormat::Json => {
            let (code, message, status) = if let Some(obs) = err.downcast_ref::<ya_obs::Error>() {
                (obs.code().to_string(), obs.message().to_string(), obs.status())
            } else if err.downcast_ref::<ConfigError>().is_some() {
                ("Config".to_string(), err.to_string(), 0)
            } else {
                ("Error".to_string(), err.to_string(), 0)
            };
            let line = serde_json::json!({
                "error": {
                    "code": code,
                    "message": message,
                    "status": status,
                }
            });
            eprintln!("{line}");
        }
    }
}
