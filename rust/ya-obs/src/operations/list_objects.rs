use reqwest::Method;

use crate::error::Error;
use crate::http::HttpClient;
use crate::models::ListedObject;
use crate::url::build_object_url;
use crate::xml::parse_list_bucket_result;

pub async fn list_objects(
    http: &HttpClient,
    bucket: &str,
    prefix: Option<&str>,
) -> Result<Vec<ListedObject>, Error> {
    let mut all = Vec::new();
    let mut marker: Option<String> = None;

    loop {
        let mut url = build_object_url(&http.config, bucket, "")?;
        {
            let mut q = url.query_pairs_mut();
            if let Some(p) = prefix {
                q.append_pair("prefix", p);
            }
            if let Some(m) = &marker {
                q.append_pair("marker", m);
            }
        }

        let resp = http
            .send(Method::GET, url, Vec::new(), bytes::Bytes::new())
            .await?;
        let body = resp.text().await.map_err(Error::Transport)?;
        let page = parse_list_bucket_result(&body)?;

        let last_key = page.objects.last().map(|o| o.key.clone());
        all.extend(page.objects);

        if !page.is_truncated {
            return Ok(all);
        }
        marker = page.next_marker.or(last_key);
        if marker.is_none() {
            return Ok(all);
        }
    }
}
