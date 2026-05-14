use bytes::Bytes;

use crate::config::ClientConfig;
use crate::error::Error;
use crate::http::HttpClient;
use crate::models::PutObjectResponse;
use crate::operations;

pub struct Client {
    http: HttpClient,
}

impl Client {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let http = HttpClient::new(config)?;
        Ok(Self { http })
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
    ) -> Result<PutObjectResponse, Error> {
        operations::put_object::put_object(&self.http, bucket, key, body).await
    }
}
