use bytes::Bytes;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::{
    AddressingStyle, Client, ClientConfig, Credentials, ObjectAcl, PutObjectOptions, SigningVersion,
};

fn client(server: &MockServer, signing: SigningVersion) -> Client {
    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("test-ak", "test-sk"))
        .with_addressing_style(AddressingStyle::Path)
        .with_signing_version(signing);
    cfg.region = Some("cn-southwest-2".into());
    Client::new(cfg).unwrap()
}
async fn unversioned(server: &MockServer) {
    Mock::given(method("GET"))
        .and(|req: &wiremock::Request| req.url.query() == Some("versioning"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<VersioningConfiguration xmlns=\"http://obs.myhuaweicloud.com/doc/2015-06-30/\"/>",
        ))
        .mount(server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/bucket/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
}
#[tokio::test]
async fn publishing_headers_and_stat_roundtrip_for_both_signers() {
    for signing in [SigningVersion::V2, SigningVersion::V4] {
        let server = MockServer::start().await;
        unversioned(&server).await;
        Mock::given(method("PUT"))
            .and(path("/bucket/skill.zip"))
            .and(header("x-obs-forbid-overwrite", "true"))
            .and(header("x-obs-acl", "public-read"))
            .and(header("content-type", "application/zip"))
            .and(|req: &wiremock::Request| {
                req.headers
                    .get("cache-control")
                    .is_some_and(|v| v == "public, max-age=31536000, immutable")
            })
            .and(header(
                "content-disposition",
                "attachment; filename=\"skill.zip\"",
            ))
            .and(header("x-obs-meta-sha256", "abc"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .and(path("/bucket/skill.zip"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/zip")
                    .insert_header("cache-control", "public, max-age=31536000, immutable")
                    .insert_header("content-disposition", "attachment; filename=\"skill.zip\"")
                    .insert_header("x-obs-meta-sha256", "abc"),
            )
            .mount(&server)
            .await;
        let client = client(&server, signing);
        let options = PutObjectOptions {
            no_clobber: true,
            acl: Some(ObjectAcl::PublicRead),
            content_type: Some("application/zip".into()),
            cache_control: Some("public, max-age=31536000, immutable".into()),
            content_disposition: Some("attachment; filename=\"skill.zip\"".into()),
            metadata: [("sha256".into(), "abc".into())].into(),
        };
        let result = client
            .put_object_with_options("bucket", "skill.zip", Bytes::from_static(b"zip"), &options)
            .await;
        assert!(result.is_ok(), "{result:?}");
        let meta = client.head_object("bucket", "skill.zip").await.unwrap();
        assert_eq!(meta.content_type, options.content_type);
        assert_eq!(meta.cache_control, options.cache_control);
        assert_eq!(meta.content_disposition, options.content_disposition);
        assert_eq!(meta.metadata, options.metadata);
    }
}
#[tokio::test]
async fn unsupported_bucket_states_never_put() {
    for body in [
        "<VersioningConfiguration><Status>Enabled</Status></VersioningConfiguration>",
        "<VersioningConfiguration><Status>Suspended</Status></VersioningConfiguration>",
        "<VersioningConfiguration><Status>Unknown</Status></VersioningConfiguration>",
        "<Other/>",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        let err = client(&server, SigningVersion::V4)
            .put_object_with_options(
                "bucket",
                "key",
                Bytes::new(),
                &PutObjectOptions {
                    no_clobber: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ya_obs::Error::Unsupported(_)), "{err:?}");
        assert!(server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| r.method != "PUT"));
    }
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("x-obs-fs-file-interface", "Enabled"),
        )
        .mount(&server)
        .await;
    let err = client(&server, SigningVersion::V4)
        .put_object_with_options(
            "bucket",
            "key",
            Bytes::new(),
            &PutObjectOptions {
                no_clobber: true,
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ya_obs::Error::Unsupported(_)));
    assert!(server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .all(|r| r.method != "PUT"));
}
#[tokio::test]
async fn permission_denied_during_preflight_never_puts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let err = client(&server, SigningVersion::V4)
        .put_object_with_options(
            "bucket",
            "key",
            Bytes::new(),
            &PutObjectOptions {
                no_clobber: true,
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "AccessDenied");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
#[tokio::test]
async fn retries_preserve_protection_and_report_conflict() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let server = MockServer::start().await;
    unversioned(&server).await;
    let count = Arc::new(AtomicUsize::new(0));
    Mock::given(method("PUT"))
        .and(header("x-obs-forbid-overwrite", "true"))
        .respond_with(move |_: &wiremock::Request| {
            let n = count.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(if n == 0 { 503 } else { 409 })
        })
        .expect(2)
        .mount(&server)
        .await;
    let err = client(&server, SigningVersion::V4)
        .put_object_with_options(
            "bucket",
            "key",
            Bytes::from_static(b"zip"),
            &PutObjectOptions {
                no_clobber: true,
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.status(), 409);
    assert_eq!(err.code(), "Conflict");
}
#[tokio::test]
async fn object_acl_only_changes_requested_object() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/bucket/key"))
        .and(|req: &wiremock::Request| req.url.query() == Some("acl"))
        .and(header("x-obs-acl", "public-read"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, SigningVersion::V2);
    client
        .set_object_acl("bucket", "key", ObjectAcl::PublicRead)
        .await
        .unwrap();
    assert!(client
        .set_object_acl("bucket", "", ObjectAcl::PublicRead)
        .await
        .is_err());
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].body.is_empty());
}
#[tokio::test]
async fn invalid_headers_and_multipart_options_rejected_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, SigningVersion::V4);
    for options in [
        PutObjectOptions {
            content_type: Some("zip\r\nx-host: evil".into()),
            ..Default::default()
        },
        PutObjectOptions {
            metadata: [("bad:key".into(), "value".into())].into(),
            ..Default::default()
        },
    ] {
        assert!(client
            .put_object_with_options("bucket", "key", Bytes::new(), &options)
            .await
            .is_err());
    }
    let err = client
        .put_object_with_options(
            "bucket",
            "key",
            Bytes::from(vec![0; 100 * 1024 * 1024]),
            &PutObjectOptions {
                no_clobber: true,
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ya_obs::Error::Unsupported(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn redirects_do_not_report_upload_success_or_forward_signed_requests() {
    let server = MockServer::start().await;
    let destination = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", destination.uri()))
        .mount(&server)
        .await;
    let err = client(&server, SigningVersion::V4)
        .put_object("bucket", "key", Bytes::new())
        .await
        .unwrap_err();
    assert_eq!(err.status(), 307);
    assert!(destination.received_requests().await.unwrap().is_empty());
}
