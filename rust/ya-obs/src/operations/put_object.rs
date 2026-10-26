use bytes::Bytes;
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::multipart;
use crate::url::build_object_url;

const MULTIPART_THRESHOLD: usize = 100 * 1024 * 1024;
const DEFAULT_PART_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_CONCURRENCY: usize = 4;

pub async fn put_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Result<PutObjectResponse, Error> {
    put_object_with_options(http, bucket, key, body, &crate::PutObjectOptions::default()).await
}

pub async fn put_object_with_options(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
    options: &crate::PutObjectOptions,
) -> Result<PutObjectResponse, Error> {
    let has_options = options != &crate::PutObjectOptions::default();
    if has_options && (key.is_empty() || body.len() >= MULTIPART_THRESHOLD) {
        return Err(Error::Unsupported(
            "upload options require a nonempty key and a single PUT below 100 MiB".into(),
        ));
    }
    if body.len() >= MULTIPART_THRESHOLD {
        return multipart::upload(
            http,
            bucket,
            key,
            body,
            DEFAULT_PART_SIZE,
            DEFAULT_CONCURRENCY,
        )
        .await;
    }

    let url = build_object_url(&http.config, bucket, key)?;
    let mut headers = vec![("Content-Length".to_string(), body.len().to_string())];
    for (name, value) in [
        ("Content-Type", &options.content_type),
        ("Cache-Control", &options.cache_control),
        ("Content-Disposition", &options.content_disposition),
    ] {
        if let Some(value) = value {
            headers.push((name.into(), value.clone()));
        }
    }
    if let Some(acl) = options.acl {
        headers.push(("x-obs-acl".into(), acl.as_str().into()));
    }
    if options.no_clobber {
        headers.push(("x-obs-forbid-overwrite".into(), "true".into()));
    }
    for (key, value) in &options.metadata {
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(Error::Config(
                "metadata keys must use lowercase ASCII letters, digits, hyphens or underscores"
                    .into(),
            ));
        }
        headers.push((format!("x-obs-meta-{key}"), value.clone()));
    }
    for (_, value) in &headers {
        reqwest::header::HeaderValue::from_str(value)
            .map_err(|_| Error::Config("invalid upload header value".into()))?;
    }
    if options.no_clobber {
        super::publishing::check_no_clobber(http, bucket).await?;
    }
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
