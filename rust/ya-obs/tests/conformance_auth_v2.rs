mod support;

use std::collections::BTreeMap;
use ya_obs::signer::v2::{authorization_header, string_to_sign};

#[test]
fn v2_header_basic_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v2_header_basic");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = input["obs_headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let actual = string_to_sign(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v2_header_content_md5_string_to_sign_matches_vector() {
    let v = support::load_vector("auth", "v2_header_content_md5");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = input["obs_headers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let actual = string_to_sign(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
    );

    assert_eq!(actual, v["expected"]["string_to_sign"].as_str().unwrap());
}

#[test]
fn v2_header_basic_authorization_prefix() {
    let v = support::load_vector("auth", "v2_header_basic");
    let input = &v["input"];

    let obs_headers: BTreeMap<String, String> = BTreeMap::new();

    let auth = authorization_header(
        input["method"].as_str().unwrap(),
        input["content_md5"].as_str().unwrap(),
        input["content_type"].as_str().unwrap(),
        input["date"].as_str().unwrap(),
        &obs_headers,
        input["bucket"].as_str().unwrap(),
        input["key"].as_str().unwrap(),
        input["access_key"].as_str().unwrap(),
        input["secret_key"].as_str().unwrap(),
    );

    assert!(auth.starts_with(v["expected"]["authorization_prefix"].as_str().unwrap()));
}
