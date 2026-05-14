use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::url::build_object_url;

pub async fn delete_object(
    http: &HttpClient,
    bucket: &str,
    key: &str,
) -> Result<(), Error> {
    let url = build_object_url(&http.config, bucket, key)?;
    http.send(Method::DELETE, url, Vec::new(), bytes::Bytes::new()).await?;
    Ok(())
}
