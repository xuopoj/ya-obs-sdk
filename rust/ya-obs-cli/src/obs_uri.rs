use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsUri {
    pub bucket: String,
    pub key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ObsUriError {
    #[error("expected obs:// scheme, got {0}")]
    BadScheme(String),
    #[error("missing bucket in {0}")]
    NoBucket(String),
}

impl FromStr for ObsUri {
    type Err = ObsUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s.strip_prefix("obs://")
            .ok_or_else(|| ObsUriError::BadScheme(s.to_string()))?;
        let (bucket, key) = match rest.split_once('/') {
            Some((b, k)) => (b, k),
            None => (rest, ""),
        };
        if bucket.is_empty() {
            return Err(ObsUriError::NoBucket(s.to_string()));
        }
        Ok(ObsUri { bucket: bucket.to_string(), key: key.to_string() })
    }
}
