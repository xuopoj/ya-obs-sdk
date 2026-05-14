use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::url::build_object_url;
use crate::xml::{parse_initiate_multipart_result, serialize_complete_multipart};

async fn initiate(http: &HttpClient, bucket: &str, key: &str) -> Result<String, Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploads", "");
    let resp = http
        .send(Method::POST, url, Vec::new(), Bytes::new())
        .await?;
    let body = resp.text().await.map_err(Error::Transport)?;
    Ok(parse_initiate_multipart_result(&body)?.upload_id)
}

async fn upload_part(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: u32,
    body: Bytes,
) -> Result<(u32, String), Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut()
        .append_pair("partNumber", &part_number.to_string())
        .append_pair("uploadId", upload_id);
    let headers = vec![("Content-Length".to_string(), body.len().to_string())];
    let resp = http.send(Method::PUT, url, headers, body).await?;
    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    Ok((part_number, etag))
}

async fn complete(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    upload_id: &str,
    parts: Vec<(u32, String)>,
) -> Result<PutObjectResponse, Error> {
    let xml = serialize_complete_multipart(&parts);
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploadId", upload_id);
    let headers = vec![
        ("Content-Type".to_string(), "application/xml".to_string()),
        ("Content-Length".to_string(), xml.len().to_string()),
    ];
    let resp = http
        .send(Method::POST, url, headers, Bytes::from(xml))
        .await?;
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

async fn abort(http: &HttpClient, bucket: &str, key: &str, upload_id: &str) -> Result<(), Error> {
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.query_pairs_mut().append_pair("uploadId", upload_id);
    http.send(Method::DELETE, url, Vec::new(), Bytes::new())
        .await?;
    Ok(())
}

/// Upload `body` as a multipart upload, splitting into `part_size` chunks and
/// running `concurrency` part uploads in parallel.
pub async fn upload(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    body: Bytes,
    part_size: usize,
    concurrency: usize,
) -> Result<PutObjectResponse, Error> {
    let upload_id = initiate(http, bucket, key).await?;

    let mut chunks: Vec<(u32, Bytes)> = Vec::new();
    let mut offset = 0usize;
    let mut n: u32 = 1;
    while offset < body.len() {
        let end = (offset + part_size).min(body.len());
        chunks.push((n, body.slice(offset..end)));
        offset = end;
        n += 1;
    }

    let result: Result<Vec<(u32, String)>, Error> = async {
        let mut in_flight: FuturesUnordered<_> = FuturesUnordered::new();
        let mut iter = chunks.into_iter();
        for _ in 0..concurrency {
            if let Some((pn, b)) = iter.next() {
                in_flight.push(upload_part(http, bucket, key, &upload_id, pn, b));
            }
        }
        let mut finished: Vec<(u32, String)> = Vec::new();
        while let Some(res) = in_flight.next().await {
            finished.push(res?);
            if let Some((pn, b)) = iter.next() {
                in_flight.push(upload_part(http, bucket, key, &upload_id, pn, b));
            }
        }
        finished.sort_by_key(|(n, _)| *n);
        Ok(finished)
    }
    .await;

    match result {
        Ok(parts) => complete(http, bucket, key, &upload_id, parts).await,
        Err(e) => {
            let _ = abort(http, bucket, key, &upload_id).await;
            Err(e)
        }
    }
}
