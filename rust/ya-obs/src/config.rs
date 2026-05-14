use std::time::Duration;

use crate::credentials::Credentials;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningVersion {
    V4,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingStyle {
    Auto,
    Virtual,
    Path,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub credentials: Option<Credentials>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub signing_version: SigningVersion,
    pub addressing_style: AddressingStyle,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub user_agent: String,
}

impl ClientConfig {
    pub fn for_region(region: impl Into<String>) -> Self {
        Self {
            credentials: None,
            region: Some(region.into()),
            endpoint: None,
            signing_version: SigningVersion::V4,
            addressing_style: AddressingStyle::Auto,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            user_agent: format!("ya-obs/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn for_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            credentials: None,
            region: None,
            endpoint: Some(endpoint.into()),
            signing_version: SigningVersion::V4,
            addressing_style: AddressingStyle::Auto,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            user_agent: format!("ya-obs/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn with_credentials(mut self, c: Credentials) -> Self {
        self.credentials = Some(c);
        self
    }

    pub fn with_signing_version(mut self, v: SigningVersion) -> Self {
        self.signing_version = v;
        self
    }

    pub fn with_addressing_style(mut self, s: AddressingStyle) -> Self {
        self.addressing_style = s;
        self
    }
}
