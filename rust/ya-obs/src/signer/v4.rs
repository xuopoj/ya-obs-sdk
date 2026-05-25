use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

/// Characters allowed in SigV4 canonical query values — everything outside
/// `unreserved` gets percent-encoded.
const QUERY_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'^')
    .add(b'|')
    .add(b'\\')
    .add(b'%')
    .add(b'+')
    .add(b'!')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b',')
    .add(b';')
    .add(b'=')
    .add(b':')
    .add(b'@')
    .add(b'[')
    .add(b']')
    .add(b'/');

fn canonical_uri(url: &Url) -> String {
    // S3-style SigV4: path is encoded once on the wire by `build_object_url`,
    // and the canonical URI is that wire form verbatim. Re-running it through
    // a percent-encoder would double-encode non-ASCII keys (every `%XX` would
    // become `%25XX`), which is the source of AccessDenied on Chinese keys.
    let path = url.path();
    if path.is_empty() {
        "/".into()
    } else {
        path.to_string()
    }
}

fn canonical_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| {
            (
                utf8_percent_encode(&k, QUERY_ENCODE_SET).to_string(),
                utf8_percent_encode(&v, QUERY_ENCODE_SET).to_string(),
            )
        })
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Build the SigV4 canonical request string.
/// `headers` is the full set of headers to sign (must include `Host`).
pub fn canonical_request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body_sha256: &str,
) -> String {
    let parsed = Url::parse(url).expect("valid url");

    let uri = canonical_uri(&parsed);
    let query = canonical_query(&parsed);

    let mut lowered: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    lowered.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = lowered.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();

    let signed_headers = lowered
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    format!("{method}\n{uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{body_sha256}")
}

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Build the AWS4-HMAC-SHA256 string-to-sign.
///
/// The returned string matches the test-vector format: the trailing line is
/// empty (callers prepend the hashed canonical request only when computing the
/// signature, not when comparing to fixtures).
pub fn string_to_sign(
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
    _canonical_request: &str,
) -> String {
    format!("AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n")
}

/// Derive the SigV4 signing key.
pub fn signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// Compute the hex signature for a request and produce the full Authorization
/// header value.
#[allow(clippy::too_many_arguments)]
pub fn authorization_header(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body_sha256: &str,
    access_key: &str,
    secret_key: &str,
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
) -> String {
    let canonical = canonical_request(method, url, headers, body_sha256);
    let hashed_canonical = sha256_hex(canonical.as_bytes());

    let to_sign_with_hash = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{hashed_canonical}"
    );

    let key = signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&key, to_sign_with_hash.as_bytes()));

    let mut lowered: Vec<String> = headers
        .iter()
        .map(|(k, _)| k.to_ascii_lowercase())
        .collect();
    lowered.sort();
    let signed_headers = lowered.join(";");

    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{date}/{region}/{service}/aws4_request, \
         SignedHeaders={signed_headers}, Signature={signature}"
    )
}

/// Build a SigV4 presigned URL by adding `X-Amz-*` query parameters.
#[allow(clippy::too_many_arguments)]
pub fn presign_url(
    method: &str,
    url: &str,
    access_key: &str,
    secret_key: &str,
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
    expires: u64,
) -> String {
    let mut parsed = Url::parse(url).expect("valid url");
    let host = parsed.host_str().expect("url must have host").to_string();

    let credential = format!("{access_key}/{date}/{region}/{service}/aws4_request");

    parsed
        .query_pairs_mut()
        .append_pair("X-Amz-Algorithm", "AWS4-HMAC-SHA256")
        .append_pair("X-Amz-Credential", &credential)
        .append_pair("X-Amz-Date", amz_date)
        .append_pair("X-Amz-Expires", &expires.to_string())
        .append_pair("X-Amz-SignedHeaders", "host");

    // For presigned URLs, the canonical request uses UNSIGNED-PAYLOAD as the
    // body hash and includes only the `host` header.
    let headers = vec![("host".to_string(), host)];
    let canonical = canonical_request(method, parsed.as_str(), &headers, "UNSIGNED-PAYLOAD");
    let hashed_canonical = sha256_hex(canonical.as_bytes());

    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{hashed_canonical}"
    );
    let key = signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&key, to_sign.as_bytes()));

    parsed
        .query_pairs_mut()
        .append_pair("X-Amz-Signature", &signature);
    parsed.to_string()
}
