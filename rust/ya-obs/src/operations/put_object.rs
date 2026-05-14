use bytes::Bytes;
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::url::build_object_url;

pub async fn put_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Result<PutObjectResponse, Error> {
    let url = build_object_url(&http.config, bucket, key)?;

    let headers = vec![
        ("Content-Length".to_string(), body.len().to_string()),
    ];

    let resp = http.send(Method::PUT, url, headers, body).await?;

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let request_id = resp
        .headers()
        .get("x-obs-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    Ok(PutObjectResponse { etag, request_id })
}
