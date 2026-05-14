use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
}

impl Credentials {
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self { access_key: access_key.into(), secret_key: secret_key.into() }
    }

    pub fn from_env() -> Result<Self, Error> {
        let access_key = std::env::var("HUAWEICLOUD_SDK_AK")
            .map_err(|_| Error::Config("HUAWEICLOUD_SDK_AK not set".into()))?;
        let secret_key = std::env::var("HUAWEICLOUD_SDK_SK")
            .map_err(|_| Error::Config("HUAWEICLOUD_SDK_SK not set".into()))?;
        Ok(Self { access_key, secret_key })
    }
}
