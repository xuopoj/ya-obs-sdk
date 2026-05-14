use quick_xml::de::from_str;
use serde::Deserialize;

use crate::models::{ListBucketResult, ListedObject};

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
