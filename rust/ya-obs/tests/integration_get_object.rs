use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

#[tokio::test]
async fn get_object_returns_body_and_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/my-bucket/hello.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"abc123\"")
                .insert_header("Content-Type", "text/plain")
                .set_body_bytes(b"hello world".as_ref()),
        )
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let resp = client.get_object("my-bucket", "hello.txt").await.unwrap();

    assert_eq!(resp.etag, "\"abc123\"");
    assert_eq!(resp.content_type.as_deref(), Some("text/plain"));

    let bytes = resp.body.read_to_end().await.unwrap();
    assert_eq!(&bytes[..], b"hello world");
}
