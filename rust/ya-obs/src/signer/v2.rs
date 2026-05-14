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

use url::Url;

/// OBS V2 presigned URL. For presigning, the "Date" line in the canonical
/// string is the Unix-timestamp expiry, and Content-MD5/Content-Type are empty.
///
/// Returns `(url, string_to_sign)` so callers (and tests) can verify the STS.
#[allow(clippy::too_many_arguments)]
pub fn presign_url(
    method: &str,
    url: &str,
    bucket: &str,
    key: &str,
    expires_unix: u64,
    obs_headers: &BTreeMap<String, String>,
    access_key: &str,
    secret_key: &str,
) -> (String, String) {
    let sts = string_to_sign(method, "", "", &expires_unix.to_string(), obs_headers, bucket, key);
    let signature = sign(secret_key, &sts);

    let mut parsed = Url::parse(url).expect("valid url");
    parsed.query_pairs_mut()
        .append_pair("AccessKeyId", access_key)
        .append_pair("Expires", &expires_unix.to_string())
        .append_pair("Signature", &signature);

    (parsed.to_string(), sts)
}
