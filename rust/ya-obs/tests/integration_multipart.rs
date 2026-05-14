use bytes::Bytes;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

const INITIATE_XML: &str = r#"<?xml version="1.0"?>
<InitiateMultipartUploadResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Bucket>b</Bucket><Key>k</Key><UploadId>upload-xyz</UploadId>
</InitiateMultipartUploadResult>"#;

#[tokio::test]
async fn multipart_uploads_two_parts_and_completes() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/b/k"))
        .and(query_param("uploads", ""))
        .respond_with(ResponseTemplate::new(200).set_body_string(INITIATE_XML))
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/b/k"))
        .and(query_param("partNumber", "1"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"p1\""))
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/b/k"))
        .and(query_param("partNumber", "2"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"p2\""))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/b/k"))
        .and(query_param("uploadId", "upload-xyz"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"final\""))
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let mut body = vec![0u8; 10 * 1024 * 1024];
    body[0] = 1;
    let resp = client
        .put_object_multipart("b", "k", Bytes::from(body), 5 * 1024 * 1024, 2)
        .await
        .unwrap();
    assert_eq!(resp.etag, "\"final\"");
}
