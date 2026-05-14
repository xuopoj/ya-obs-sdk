mod support;

use ya_obs::xml::parse_list_bucket_result;

#[test]
fn list_objects_response_parses_to_expected_objects() {
    let v = support::load_vector("xml", "list_objects_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let result = parse_list_bucket_result(xml).expect("parse ok");

    assert_eq!(result.name, v["expected"]["name"].as_str().unwrap());
    assert_eq!(result.is_truncated, v["expected"]["is_truncated"].as_bool().unwrap());
    assert!(result.next_marker.is_none());

    let expected_objects = v["expected"]["objects"].as_array().unwrap();
    assert_eq!(result.objects.len(), expected_objects.len());

    for (got, want) in result.objects.iter().zip(expected_objects) {
        assert_eq!(got.key, want["key"].as_str().unwrap());
        assert_eq!(got.etag, want["etag"].as_str().unwrap());
        assert_eq!(got.size, want["size"].as_u64().unwrap());
        assert_eq!(got.last_modified, want["last_modified"].as_str().unwrap());
    }
}

use ya_obs::xml::{
    parse_error_response, parse_initiate_multipart_result, serialize_complete_multipart,
};

#[test]
fn error_response_parses_to_expected_fields() {
    let v = support::load_vector("xml", "error_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let err = parse_error_response(xml).expect("parse ok");

    assert_eq!(err.code, v["expected"]["code"].as_str().unwrap());
    assert_eq!(err.message, v["expected"]["message"].as_str().unwrap());
    assert_eq!(err.request_id, v["expected"]["request_id"].as_str().unwrap());
    assert_eq!(err.host_id, v["expected"]["host_id"].as_str().unwrap());
}

#[test]
fn initiate_multipart_result_parses_to_expected_fields() {
    let v = support::load_vector("xml", "initiate_multipart_response");
    let xml = v["input"]["xml"].as_str().unwrap();

    let r = parse_initiate_multipart_result(xml).expect("parse ok");

    assert_eq!(r.bucket, v["expected"]["bucket"].as_str().unwrap());
    assert_eq!(r.key, v["expected"]["key"].as_str().unwrap());
    assert_eq!(r.upload_id, v["expected"]["upload_id"].as_str().unwrap());
}

#[test]
fn complete_multipart_request_serializes_to_expected_xml() {
    let v = support::load_vector("xml", "complete_multipart_request");
    let parts: Vec<(u32, String)> = v["input"]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (
            p["part_number"].as_u64().unwrap() as u32,
            p["etag"].as_str().unwrap().to_string(),
        ))
        .collect();

    let actual = serialize_complete_multipart(&parts);
    assert_eq!(actual, v["expected"]["xml"].as_str().unwrap());
}
