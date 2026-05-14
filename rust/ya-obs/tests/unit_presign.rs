use std::collections::HashMap;
use ya_obs::client::Client;
use ya_obs::config::{AddressingStyle, ClientConfig, SigningVersion};
use ya_obs::credentials::Credentials;

fn client(sv: SigningVersion) -> Client {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_credentials(Credentials::new("AK", "SK"))
        .with_signing_version(sv)
        .with_addressing_style(AddressingStyle::Virtual);
    Client::new(cfg).unwrap()
}

#[test]
fn presign_v4_includes_required_params() {
    let url = client(SigningVersion::V4)
        .presign_get_object("my-bucket", "k.jpg", 3600).unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    let qs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    for k in ["X-Amz-Algorithm", "X-Amz-Credential", "X-Amz-Date", "X-Amz-Expires", "X-Amz-SignedHeaders", "X-Amz-Signature"] {
        assert!(qs.contains_key(k), "missing {k}");
    }
}

#[test]
fn presign_v2_includes_required_params() {
    let url = client(SigningVersion::V2)
        .presign_get_object("my-bucket", "k.jpg", 3600).unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    let qs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    for k in ["AccessKeyId", "Expires", "Signature"] {
        assert!(qs.contains_key(k), "missing {k}");
    }
}
