use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::GetObjectResponse;
use crate::streaming::StreamingBody;
use crate::url::build_object_url;

pub async fn get_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<GetObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    let resp = http
        .send(Method::GET, url, Vec::new(), bytes::Bytes::new())
        .await?;

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let content_length = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let request_id = resp
        .headers()
        .get("x-obs-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    Ok(GetObjectResponse {
        body: StreamingBody::new(resp),
        content_type,
        content_length,
        etag,
        request_id,
    })
}
