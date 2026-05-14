use bytes::Bytes;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

#[tokio::test]
async fn put_object_sends_signed_put_and_returns_etag() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/my-bucket/hello.txt"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"abc123\""))
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let resp = client
        .put_object("my-bucket", "hello.txt", Bytes::from_static(b"hi"))
        .await
        .unwrap();

    assert_eq!(resp.etag, "\"abc123\"");
}
