#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBucketResult {
    pub name: String,
    pub prefix: Option<String>,
    pub max_keys: u32,
    pub is_truncated: bool,
    pub next_marker: Option<String>,
    pub objects: Vec<ListedObject>,
    /// Common prefixes returned when a delimiter is in effect (e.g. ["foo/", "bar/"]).
    pub common_prefixes: Vec<String>,
}

/// Combined result of a delimited list_objects call across all pages.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListObjectsResult {
    pub objects: Vec<ListedObject>,
    pub common_prefixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedObject {
    pub key: String,
    pub etag: String,
    pub size: u64,
    /// ISO-8601 with offset, e.g. `2024-01-15T10:30:00+00:00`.
    pub last_modified: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedBucket {
    pub name: String,
    /// ISO-8601 with offset, e.g. `2024-01-15T10:30:00+00:00`.
    pub creation_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub host_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateMultipartResult {
    pub bucket: String,
    pub key: String,
    pub upload_id: String,
}

#[derive(Debug, Clone)]
pub struct PutObjectResponse {
    pub etag: String,
    pub request_id: Option<String>,
}

pub struct GetObjectResponse {
    pub body: crate::streaming::StreamingBody,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub etag: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeadObjectResponse {
    pub etag: String,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub request_id: Option<String>,
}
