use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

const LIST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Owner><ID>owner-abc</ID><DisplayName>example</DisplayName></Owner>
  <Buckets>
    <Bucket><Name>bucket-a</Name><CreationDate>2024-01-15T10:30:00.000Z</CreationDate><Location>cn-north-4</Location></Bucket>
    <Bucket><Name>bucket-b</Name><CreationDate>2024-02-20T08:00:00.000Z</CreationDate></Bucket>
  </Buckets>
</ListAllMyBucketsResult>"#;

#[tokio::test]
async fn list_buckets_returns_bucket_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIST_XML))
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let buckets = client.list_buckets().await.unwrap();

    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0].name, "bucket-a");
    assert_eq!(buckets[1].name, "bucket-b");
}
