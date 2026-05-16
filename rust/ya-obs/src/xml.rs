use quick_xml::de::from_str;
use serde::Deserialize;

use crate::models::{
    ErrorResponse, InitiateMultipartResult, ListBucketResult, ListedBucket, ListedObject,
};

#[derive(Debug, Deserialize)]
struct RawContents {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "LastModified")]
    last_modified: String,
    #[serde(rename = "ETag")]
    etag: String,
    #[serde(rename = "Size")]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct RawListBucketResult {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Prefix", default)]
    prefix: Option<String>,
    #[serde(rename = "MaxKeys", default)]
    max_keys: u32,
    #[serde(rename = "IsTruncated")]
    is_truncated: bool,
    #[serde(rename = "NextMarker", default)]
    next_marker: Option<String>,
    #[serde(rename = "Contents", default)]
    contents: Vec<RawContents>,
}

/// Normalize OBS `LastModified` (`2024-01-15T10:30:00.000Z`) into the
/// `2024-01-15T10:30:00+00:00` form the test vectors expect.
fn normalize_last_modified(raw: &str) -> String {
    let no_frac = raw.split('.').next().unwrap_or(raw);
    if raw.ends_with('Z') {
        format!("{no_frac}+00:00")
    } else {
        no_frac.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct RawBucketEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "CreationDate")]
    creation_date: String,
}

#[derive(Debug, Deserialize)]
struct RawBuckets {
    #[serde(rename = "Bucket", default)]
    bucket: Vec<RawBucketEntry>,
}

#[derive(Debug, Deserialize)]
struct RawListAllMyBucketsResult {
    #[serde(rename = "Buckets", default)]
    buckets: Option<RawBuckets>,
}

pub fn parse_list_all_my_buckets(xml: &str) -> Result<Vec<ListedBucket>, quick_xml::DeError> {
    let raw: RawListAllMyBucketsResult = from_str(xml)?;
    Ok(raw
        .buckets
        .map(|b| b.bucket)
        .unwrap_or_default()
        .into_iter()
        .map(|b| ListedBucket {
            name: b.name,
            creation_date: normalize_last_modified(&b.creation_date),
        })
        .collect())
}

pub fn parse_list_bucket_result(xml: &str) -> Result<ListBucketResult, quick_xml::DeError> {
    let raw: RawListBucketResult = from_str(xml)?;

    let objects = raw
        .contents
        .into_iter()
        .map(|c| ListedObject {
            key: c.key,
            etag: c.etag,
            size: c.size,
            last_modified: normalize_last_modified(&c.last_modified),
        })
        .collect();

    Ok(ListBucketResult {
        name: raw.name,
        prefix: raw.prefix.filter(|p| !p.is_empty()),
        max_keys: raw.max_keys,
        is_truncated: raw.is_truncated,
        next_marker: raw.next_marker.filter(|s| !s.is_empty()),
        objects,
    })
}

#[derive(Debug, Deserialize)]
struct RawError {
    #[serde(rename = "Code")]
    code: String,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "RequestId", default)]
    request_id: String,
    #[serde(rename = "HostId", default)]
    host_id: String,
}

pub fn parse_error_response(xml: &str) -> Result<ErrorResponse, quick_xml::DeError> {
    let raw: RawError = from_str(xml)?;
    Ok(ErrorResponse {
        code: raw.code,
        message: raw.message,
        request_id: raw.request_id,
        host_id: raw.host_id,
    })
}

#[derive(Debug, Deserialize)]
struct RawInitiateMultipart {
    #[serde(rename = "Bucket")]
    bucket: String,
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "UploadId")]
    upload_id: String,
}

pub fn parse_initiate_multipart_result(
    xml: &str,
) -> Result<InitiateMultipartResult, quick_xml::DeError> {
    let raw: RawInitiateMultipart = from_str(xml)?;
    Ok(InitiateMultipartResult {
        bucket: raw.bucket,
        key: raw.key,
        upload_id: raw.upload_id,
    })
}

/// Serialize a CompleteMultipartUpload body. Hand-written rather than going via
/// serde to guarantee byte-exact output (OBS rejects extra whitespace).
pub fn serialize_complete_multipart(parts: &[(u32, String)]) -> String {
    let mut out = String::from("<CompleteMultipartUpload>");
    for (n, etag) in parts {
        out.push_str(&format!(
            "<Part><PartNumber>{n}</PartNumber><ETag>{etag}</ETag></Part>"
        ));
    }
    out.push_str("</CompleteMultipartUpload>");
    out
}
