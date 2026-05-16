use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::{ListObjectsResult, ListedObject};
use crate::url::build_object_url;
use crate::xml::parse_list_bucket_result;

/// Flat list of every object under the prefix (no delimiter, recursive).
/// Backwards-compatible signature.
pub async fn list_objects(
    http: &HttpClient,
    bucket: &str,
    prefix: Option<&str>,
) -> Result<Vec<ListedObject>, Error> {
    let result = list_objects_delimited(http, bucket, prefix, None).await?;
    Ok(result.objects)
}

/// Paginated list with optional delimiter. When `delimiter` is set, the
/// returned [`ListObjectsResult`] contains both objects matching the prefix
/// up to the next delimiter AND the set of "common prefixes" (subdirectories).
pub async fn list_objects_delimited(
    http: &HttpClient,
    bucket: &str,
    prefix: Option<&str>,
    delimiter: Option<&str>,
) -> Result<ListObjectsResult, Error> {
    let mut out = ListObjectsResult::default();
    let mut marker: Option<String> = None;

    loop {
        let mut url = build_object_url(&http.config, bucket, "")?;
        {
            let mut q = url.query_pairs_mut();
            if let Some(p) = prefix {
                q.append_pair("prefix", p);
            }
            if let Some(d) = delimiter {
                q.append_pair("delimiter", d);
            }
            if let Some(m) = &marker {
                q.append_pair("marker", m);
            }
        }

        let resp = http
            .send(Method::GET, url, Vec::new(), bytes::Bytes::new())
            .await?;
        let body = resp.text().await.map_err(Error::Transport)?;
        let page = parse_list_bucket_result(&body)?;

        let last_key = page.objects.last().map(|o| o.key.clone());
        let last_prefix = page.common_prefixes.last().cloned();
        out.objects.extend(page.objects);
        out.common_prefixes.extend(page.common_prefixes);

        if !page.is_truncated {
            return Ok(out);
        }
        // Prefer the server's NextMarker; otherwise advance by the
        // alphabetically-later of the last key / last prefix.
        marker = page.next_marker.or_else(|| match (last_key, last_prefix) {
            (Some(k), Some(p)) => Some(if k >= p { k } else { p }),
            (Some(k), None) => Some(k),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        });
        if marker.is_none() {
            return Ok(out);
        }
    }
}
