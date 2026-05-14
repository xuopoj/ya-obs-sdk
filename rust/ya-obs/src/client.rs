use bytes::Bytes;

use crate::config::ClientConfig;
use crate::error::Error;
use crate::http::HttpClient;
use crate::models::{GetObjectResponse, HeadObjectResponse, ListedObject, PutObjectResponse};
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

    pub async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectResponse, Error> {
        operations::get_object::get_object(&self.http, bucket, key).await
    }

    pub async fn head_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<HeadObjectResponse, Error> {
        operations::head_object::head_object(&self.http, bucket, key).await
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), Error> {
        operations::delete_object::delete_object(&self.http, bucket, key).await
    }

    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<ListedObject>, Error> {
        operations::list_objects::list_objects(&self.http, bucket, prefix).await
    }

    pub fn presign_get_object(
        &self,
        bucket: &str,
        key: &str,
        expires: u64,
    ) -> Result<String, Error> {
        operations::presign::presign_get_object(&self.http.config, bucket, key, expires)
    }
}
