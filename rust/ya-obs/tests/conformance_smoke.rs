mod support;

#[test]
fn loads_v4_header_basic_vector() {
    let v = support::load_vector("auth", "v4_header_basic");
    assert_eq!(v["name"], "v4_header_basic");
    assert_eq!(v["input"]["region"], "cn-north-4");
}
