use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::BTreeMap;

type HmacSha1 = Hmac<Sha1>;

fn canonicalized_obs_headers(headers: &BTreeMap<String, String>) -> String {
    headers
        .iter()
        .filter(|(k, _)| k.to_ascii_lowercase().starts_with("x-obs-"))
        .map(|(k, v)| format!("{}:{}", k.to_ascii_lowercase(), v.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn canonicalized_resource(bucket: &str, key: &str) -> String {
    if key.is_empty() {
        format!("/{bucket}/")
    } else {
        format!("/{bucket}/{key}")
    }
}

/// OBS V2 string-to-sign for header authentication.
pub fn string_to_sign(
    method: &str,
    content_md5: &str,
    content_type: &str,
    date: &str,
    obs_headers: &BTreeMap<String, String>,
    bucket: &str,
    key: &str,
) -> String {
    let canon_headers = canonicalized_obs_headers(obs_headers);
    let resource = canonicalized_resource(bucket, key);

    let header_line = if canon_headers.is_empty() {
        String::new()
    } else {
        format!("{canon_headers}\n")
    };

    format!(
        "{method}\n{content_md5}\n{content_type}\n{date}\n{header_line}{resource}"
    )
}

fn sign(secret_key: &str, to_sign: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret_key.as_bytes()).expect("HMAC accepts any key");
    mac.update(to_sign.as_bytes());
    B64.encode(mac.finalize().into_bytes())
}

#[allow(clippy::too_many_arguments)]
pub fn authorization_header(
    method: &str,
    content_md5: &str,
    content_type: &str,
    date: &str,
    obs_headers: &BTreeMap<String, String>,
    bucket: &str,
    key: &str,
    access_key: &str,
    secret_key: &str,
) -> String {
    let to_sign = string_to_sign(method, content_md5, content_type, date, obs_headers, bucket, key);
    let signature = sign(secret_key, &to_sign);
    format!("OBS {access_key}:{signature}")
}
