mod support;

use ya_obs::signer::v4::canonical_request;

#[test]
fn v4_header_basic_canonical_request_matches_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let actual = canonical_request(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["canonical_request"].as_str().unwrap());
}

use ya_obs::signer::v4::{authorization_header, string_to_sign};

#[test]
fn v4_header_basic_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let canonical = canonical_request(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
    );

    let actual = string_to_sign(
        input["headers"]["x-amz-date"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
        &canonical,
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v4_header_basic_authorization_starts_with_expected_prefix() {
    let v = support::load_vector("auth", "v4_header_basic");
    let input = &v["input"];

    let headers: Vec<(String, String)> = input["headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, val)| (k.clone(), val.as_str().unwrap().to_string()))
        .collect();

    let auth = authorization_header(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        &headers,
        input["body_sha256"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
        input["headers"]["x-amz-date"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
    );

    let prefix = v["expected"]["authorization_prefix"].as_str().unwrap();
    assert!(
        auth.starts_with(prefix),
        "authorization header {auth:?} should start with {prefix:?}"
    );
    assert!(
        auth.contains(", Signature="),
        "missing Signature= component"
    );
}

use ya_obs::signer::v4::presign_url;

#[test]
fn v4_presign_basic_url_has_required_params_and_signature() {
    let v = support::load_vector("auth", "v4_presign_basic");
    let input = &v["input"];

    let url = presign_url(
        input["method"].as_str().unwrap(),
        input["url"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
        input["datetime"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        input["region"].as_str().unwrap(),
        input["service"].as_str().unwrap(),
        input["expires"].as_u64().unwrap(),
    );

    let parsed = url::Url::parse(&url).unwrap();
    let qs: std::collections::HashMap<String, String> = parsed.query_pairs().into_owned().collect();

    for required in v["expected"]["required_params"].as_array().unwrap() {
        let k = required.as_str().unwrap();
        assert!(qs.contains_key(k), "missing required param {k}");
    }
    assert_eq!(
        qs["X-Amz-Algorithm"],
        v["expected"]["algorithm"].as_str().unwrap()
    );
    assert_eq!(
        qs["X-Amz-Expires"],
        v["expected"]["expires"].as_str().unwrap()
    );
    assert!(!qs["X-Amz-Signature"].is_empty());
}
