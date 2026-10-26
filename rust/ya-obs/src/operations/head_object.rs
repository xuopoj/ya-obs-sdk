use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::HeadObjectResponse;
use crate::url::build_object_url;

pub async fn head_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<HeadObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    let resp = http
        .send(Method::HEAD, url, Vec::new(), bytes::Bytes::new())
        .await?;

    Ok(HeadObjectResponse {
        cache_control: resp
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
        content_disposition: resp
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
        metadata: resp
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                k.as_str()
                    .strip_prefix("x-obs-meta-")
                    .and_then(|key| v.to_str().ok().map(|v| (key.to_owned(), v.to_owned())))
            })
            .collect(),
        etag: resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string(),
        content_length: resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        content_type: resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        request_id: resp
            .headers()
            .get("x-obs-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    })
}
