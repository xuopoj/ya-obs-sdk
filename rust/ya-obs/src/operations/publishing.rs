use crate::{
    error::Error,
    http::HttpClient,
    models::{BucketVersioning, ObjectAcl},
    url::build_object_url,
};
use bytes::Bytes;
use reqwest::Method;
use serde::Deserialize;

pub async fn bucket_versioning(http: &HttpClient, bucket: &str) -> Result<BucketVersioning, Error> {
    let mut url = build_object_url(&http.config, bucket, "")?;
    url.set_query(Some("versioning"));
    let body = http
        .send(Method::GET, url, vec![], Bytes::new())
        .await?
        .text()
        .await?;
    if body.trim().is_empty() {
        return Ok(BucketVersioning::Disabled);
    }
    #[derive(Deserialize)]
    #[serde(rename = "VersioningConfiguration")]
    struct Configuration {
        #[serde(rename = "Status")]
        status: Option<String>,
    }
    let mut reader = quick_xml::Reader::from_str(&body);
    let root_is_versioning = loop {
        match reader.read_event() {
            Ok(
                quick_xml::events::Event::Start(element) | quick_xml::events::Event::Empty(element),
            ) => break element.local_name().as_ref() == b"VersioningConfiguration",
            Ok(
                quick_xml::events::Event::Decl(_)
                | quick_xml::events::Event::Comment(_)
                | quick_xml::events::Event::Text(_),
            ) => continue,
            _ => break false,
        }
    };
    if !root_is_versioning {
        return Err(Error::Unsupported(
            "unrecognized versioning response".into(),
        ));
    }
    let config: Configuration = quick_xml::de::from_str(&body)
        .map_err(|_| Error::Unsupported("invalid bucket versioning response".into()))?;
    match config.status.as_deref() {
        None | Some("") => Ok(BucketVersioning::Disabled),
        Some("Enabled") => Ok(BucketVersioning::Enabled),
        Some("Suspended") => Ok(BucketVersioning::Suspended),
        _ => Err(Error::Unsupported(
            "unrecognized bucket versioning status".into(),
        )),
    }
}

pub async fn check_no_clobber(http: &HttpClient, bucket: &str) -> Result<(), Error> {
    if bucket_versioning(http, bucket).await? != BucketVersioning::Disabled {
        return Err(Error::Unsupported(
            "no-clobber requires bucket versioning to have never been enabled".into(),
        ));
    }
    let url = build_object_url(&http.config, bucket, "")?;
    let resp = http.send(Method::HEAD, url, vec![], Bytes::new()).await?;
    match resp.headers().get("x-obs-fs-file-interface") {
        None => Ok(()),
        Some(v) if v.to_str().is_ok_and(|v| v.eq_ignore_ascii_case("Disabled")) => Ok(()),
        _ => Err(Error::Unsupported(
            "no-clobber requires an OBS bucket without a parallel filesystem interface".into(),
        )),
    }
}

pub async fn set_object_acl(
    http: &HttpClient,
    bucket: &str,
    key: &str,
    acl: ObjectAcl,
) -> Result<(), Error> {
    if key.is_empty() {
        return Err(Error::Config(
            "object ACL requires a nonempty object key".into(),
        ));
    }
    let mut url = build_object_url(&http.config, bucket, key)?;
    url.set_query(Some("acl"));
    http.send(
        Method::PUT,
        url,
        vec![("x-obs-acl".into(), acl.as_str().into())],
        Bytes::new(),
    )
    .await?;
    Ok(())
}
