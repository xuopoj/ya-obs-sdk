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
