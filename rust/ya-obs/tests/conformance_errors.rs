mod support;

use ya_obs::error::{classify_error, Error};

fn check_vector(name: &str, expected_variant_name: &str) {
    let v = support::load_vector("errors", name);
    let input = &v["input"];

    let err = classify_error(
        input["status"].as_u64().unwrap() as u16,
        input["xml"].as_str().unwrap(),
    );

    let actual_variant = match &err {
        Error::NoSuchKey { .. } => "NoSuchKey",
        Error::NoSuchBucket { .. } => "NoSuchBucket",
        Error::AccessDenied { .. } => "AccessDenied",
        Error::Client { .. } => "Client",
        Error::Server { .. } => "Server",
        _ => "Other",
    };
    assert_eq!(actual_variant, expected_variant_name);

    assert_eq!(err.code(), v["expected"]["code"].as_str().unwrap());
    assert_eq!(err.message(), v["expected"]["message"].as_str().unwrap());
    assert_eq!(err.status(), v["expected"]["status"].as_u64().unwrap() as u16);
}

#[test]
fn no_such_key_maps_to_no_such_key_variant() {
    check_vector("no_such_key", "NoSuchKey");
}

#[test]
fn no_such_bucket_maps_to_no_such_bucket_variant() {
    check_vector("no_such_bucket", "NoSuchBucket");
}

#[test]
fn access_denied_maps_to_access_denied_variant() {
    check_vector("access_denied", "AccessDenied");
}
