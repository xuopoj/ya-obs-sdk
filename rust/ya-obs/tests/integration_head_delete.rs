use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

fn make_client(server: &MockServer) -> Client {
    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());
    Client::new(cfg).unwrap()
}

#[tokio::test]
async fn head_object_returns_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/b/k"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"e\"")
                .insert_header("Content-Length", "42"),
        )
        .mount(&server)
        .await;

    let r = make_client(&server).head_object("b", "k").await.unwrap();
    assert_eq!(r.etag, "\"e\"");
    assert_eq!(r.content_length, Some(42));
}

#[tokio::test]
async fn delete_object_returns_ok_on_204() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/b/k"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    make_client(&server).delete_object("b", "k").await.unwrap();
}
