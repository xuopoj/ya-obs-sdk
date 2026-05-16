use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::ListedBucket;
use crate::url::build_service_url;
use crate::xml::parse_list_all_my_buckets;

pub async fn list_buckets(http: &HttpClient) -> Result<Vec<ListedBucket>, Error> {
    let url = build_service_url(&http.config)?;
    let resp = http
        .send(Method::GET, url, Vec::new(), bytes::Bytes::new())
        .await?;
    let body = resp.text().await.map_err(Error::Transport)?;
    parse_list_all_my_buckets(&body).map_err(Error::Xml)
}
