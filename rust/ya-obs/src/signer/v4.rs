use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

/// Characters allowed in SigV4 canonical URIs — everything outside `unreserved`
/// gets percent-encoded.
const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?')
    .add(b'`').add(b'{').add(b'}').add(b'^').add(b'|').add(b'\\')
    .add(b'%').add(b'+').add(b'!').add(b'$').add(b'&').add(b'\'')
    .add(b'(').add(b')').add(b'*').add(b',').add(b';').add(b'=')
    .add(b':').add(b'@').add(b'[').add(b']');

fn encode_path_segment(seg: &str) -> String {
    utf8_percent_encode(seg, PATH_ENCODE_SET).to_string()
}

fn canonical_uri(url: &Url) -> String {
    let path = url.path();
    if path.is_empty() {
        return "/".into();
    }
    path.split('/').map(encode_path_segment).collect::<Vec<_>>().join("/")
}

fn canonical_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| {
            (
                utf8_percent_encode(&k, PATH_ENCODE_SET).to_string(),
                utf8_percent_encode(&v, PATH_ENCODE_SET).to_string(),
            )
        })
        .collect();
    pairs.sort();
    pairs.into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
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

    let canonical_headers: String = lowered
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect();

    let signed_headers = lowered
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    format!(
        "{method}\n{uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{body_sha256}"
    )
}
