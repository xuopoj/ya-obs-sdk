#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBucketResult {
    pub name: String,
    pub prefix: Option<String>,
    pub max_keys: u32,
    pub is_truncated: bool,
    pub next_marker: Option<String>,
    pub objects: Vec<ListedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedObject {
    pub key: String,
    pub etag: String,
    pub size: u64,
    /// ISO-8601 with offset, e.g. `2024-01-15T10:30:00+00:00`.
    pub last_modified: String,
}
