use bytes::Bytes;

use crate::config::ClientConfig;
use crate::error::Error;
use crate::http::HttpClient;
use crate::models::{
    GetObjectResponse, HeadObjectResponse, ListObjectsResult, ListedBucket, ListedObject,
    PutObjectResponse,
};
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

    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<GetObjectResponse, Error> {
        operations::get_object::get_object(&self.http, bucket, key).await
    }

    pub async fn head_object(&self, bucket: &str, key: &str) -> Result<HeadObjectResponse, Error> {
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

    /// List with a delimiter. With `delimiter=Some("/")`, only objects at the
    /// current "directory" level are returned, plus a list of subdirectories
    /// via `ListObjectsResult.common_prefixes`.
    pub async fn list_objects_delimited(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        delimiter: Option<&str>,
    ) -> Result<ListObjectsResult, Error> {
        operations::list_objects::list_objects_delimited(&self.http, bucket, prefix, delimiter)
            .await
    }

    pub async fn list_buckets(&self) -> Result<Vec<ListedBucket>, Error> {
        operations::list_buckets::list_buckets(&self.http).await
    }

    pub fn presign_get_object(
        &self,
        bucket: &str,
        key: &str,
        expires: u64,
    ) -> Result<String, Error> {
        operations::presign::presign_get_object(&self.http.config, bucket, key, expires)
    }

    pub async fn put_object_multipart(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
        part_size: usize,
        concurrency: usize,
    ) -> Result<PutObjectResponse, Error> {
        crate::multipart::upload(&self.http, bucket, key, body, part_size, concurrency).await
    }
}
