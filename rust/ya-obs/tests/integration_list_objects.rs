use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig};
use ya_obs::credentials::Credentials;

const PAGE_1: &str = r#"<?xml version="1.0"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>b</Name><Prefix></Prefix><MaxKeys>1</MaxKeys>
  <IsTruncated>true</IsTruncated><NextMarker>k1</NextMarker>
  <Contents><Key>k1</Key><LastModified>2024-01-15T10:30:00.000Z</LastModified><ETag>"e1"</ETag><Size>1</Size></Contents>
</ListBucketResult>"#;

const PAGE_2: &str = r#"<?xml version="1.0"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>b</Name><Prefix></Prefix><MaxKeys>1</MaxKeys>
  <IsTruncated>false</IsTruncated>
  <Contents><Key>k2</Key><LastModified>2024-01-15T10:30:00.000Z</LastModified><ETag>"e2"</ETag><Size>2</Size></Contents>
</ListBucketResult>"#;

#[tokio::test]
async fn list_objects_collects_pages() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/b/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE_1))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/b/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE_2))
        .mount(&server)
        .await;

    let mut cfg = ClientConfig::for_endpoint(server.uri())
        .with_credentials(Credentials::new("AK", "SK"))
        .with_addressing_style(AddressingStyle::Path);
    cfg.region = Some("cn-north-4".into());

    let client = Client::new(cfg).unwrap();
    let all = client.list_objects("b", None).await.unwrap();

    let keys: Vec<&str> = all.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["k1", "k2"]);
}
