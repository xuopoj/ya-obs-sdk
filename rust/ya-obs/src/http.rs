use bytes::Bytes;
use reqwest::{Client as ReqwestClient, Method, Response};
use time::{format_description::well_known::Rfc2822, macros::format_description, OffsetDateTime};
use uuid::Uuid;

use crate::config::{ClientConfig, SigningVersion};
use crate::credentials::Credentials;
use crate::error::{classify_error, Error};
use crate::retry::{RetryDecision, RetryPolicy};
use crate::signer::{v2, v4};

pub struct HttpClient {
    inner: ReqwestClient,
    pub config: ClientConfig,
    pub retry: RetryPolicy,
}

const AMZ_DATE_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const DATE_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]");

impl HttpClient {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let inner = ReqwestClient::builder()
            .user_agent(config.user_agent.clone())
            .connect_timeout(config.connect_timeout)
            .timeout(config.read_timeout)
            .danger_accept_invalid_certs(config.tls_verify_disabled)
            .build()
            .map_err(Error::Transport)?;
        Ok(Self {
            inner,
            config,
            retry: RetryPolicy::default(),
        })
    }

    fn credentials(&self) -> Result<&Credentials, Error> {
        self.config
            .credentials
            .as_ref()
            .ok_or_else(|| Error::Config("no credentials set on client".into()))
    }

    /// Execute a signed request with retries. Body is buffered (`Bytes`) so it
    /// can be retried; streaming uploads use the multipart path.
    pub async fn send(
        &self,
        method: Method,
        url: url::Url,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) -> Result<Response, Error> {
        let mut attempts: u32 = 0;
        loop {
            let signed = self.sign_request(&method, &url, &headers, &body)?;
            let resp = self.inner.execute(signed).await.map_err(Error::Transport)?;

            let status = resp.status().as_u16();
            if status < 400 {
                return Ok(resp);
            }

            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            match self
                .retry
                .classify(status, retry_after.as_deref(), attempts)
            {
                RetryDecision::Retry(delay) => {
                    tokio::time::sleep(delay).await;
                    attempts += 1;
                    continue;
                }
                RetryDecision::DoNotRetry => {
                    let body_text = resp.text().await.unwrap_or_default();
                    return Err(classify_error(status, &body_text));
                }
            }
        }
    }

    fn sign_request(
        &self,
        method: &Method,
        url: &url::Url,
        extra_headers: &[(String, String)],
        body: &Bytes,
    ) -> Result<reqwest::Request, Error> {
        let creds = self.credentials()?;
        let now = OffsetDateTime::now_utc();

        let mut headers: Vec<(String, String)> = extra_headers.to_vec();
        headers.push((
            "Host".into(),
            url.host_str().unwrap_or_default().to_string(),
        ));
        headers.push(("x-ya-obs-client-id".into(), Uuid::new_v4().to_string()));

        let auth = match self.config.signing_version {
            SigningVersion::V4 => {
                let amz_date = now.format(AMZ_DATE_FMT).unwrap();
                let date = now.format(DATE_FMT).unwrap();
                let body_sha = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(body));
                let region = self
                    .config
                    .region
                    .as_deref()
                    .ok_or_else(|| Error::Config("region required for V4 signing".into()))?;
                headers.push(("x-amz-date".into(), amz_date.clone()));
                headers.push(("x-amz-content-sha256".into(), body_sha.clone()));

                let mut header_pairs: Vec<(String, String)> = headers.clone();
                header_pairs.sort_by_key(|a| a.0.to_ascii_lowercase());

                v4::authorization_header(
                    method.as_str(),
                    url.as_str(),
                    &header_pairs,
                    &body_sha,
                    &creds.access_key,
                    &creds.secret_key,
                    &amz_date,
                    &date,
                    region,
                    "s3",
                )
            }
            SigningVersion::V2 => {
                let date = now
                    .format(&Rfc2822)
                    .map_err(|e| Error::Config(format!("date format: {e}")))?;
                headers.push(("Date".into(), date.clone()));

                let (bucket, key) = extract_bucket_key_from_url(url);
                let obs_headers: std::collections::BTreeMap<String, String> = headers
                    .iter()
                    .filter(|(k, _)| k.to_ascii_lowercase().starts_with("x-obs-"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                v2::authorization_header(
                    method.as_str(),
                    "",
                    "",
                    &date,
                    &obs_headers,
                    &bucket,
                    &key,
                    &creds.access_key,
                    &creds.secret_key,
                )
            }
        };

        headers.push(("Authorization".into(), auth));

        let mut req = self
            .inner
            .request(method.clone(), url.clone())
            .body(body.clone());
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("host") {
                continue;
            }
            req = req.header(&k, &v);
        }
        req.build().map_err(Error::Transport)
    }
}

/// Extract `(bucket, key)` from a built OBS URL. Path-style URLs look like
/// `https://obs.<region>.myhuaweicloud.com/<bucket>/<key>`; virtual-host URLs
/// look like `https://<bucket>.obs.<region>.myhuaweicloud.com/<key>`.
fn extract_bucket_key_from_url(url: &url::Url) -> (String, String) {
    let host = url.host_str().unwrap_or_default();
    let path = url.path().trim_start_matches('/');

    if let Some((first_label, rest)) = host.split_once('.') {
        if rest.starts_with("obs.") {
            return (first_label.to_string(), path.to_string());
        }
    }

    match path.split_once('/') {
        Some((b, k)) => (b.to_string(), k.to_string()),
        None => (path.to_string(), String::new()),
    }
}
