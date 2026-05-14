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
