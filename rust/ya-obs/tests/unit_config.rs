use ya_obs::config::{AddressingStyle, ClientConfig, SigningVersion};
use ya_obs::credentials::Credentials;
use ya_obs::url::build_object_url;

#[test]
fn credentials_from_env_reads_huaweicloud_vars() {
    let old_ak = std::env::var("HUAWEICLOUD_SDK_AK").ok();
    let old_sk = std::env::var("HUAWEICLOUD_SDK_SK").ok();
    std::env::set_var("HUAWEICLOUD_SDK_AK", "test-ak");
    std::env::set_var("HUAWEICLOUD_SDK_SK", "test-sk");

    let c = Credentials::from_env().expect("env credentials");
    assert_eq!(c.access_key, "test-ak");
    assert_eq!(c.secret_key, "test-sk");

    match old_ak { Some(v) => std::env::set_var("HUAWEICLOUD_SDK_AK", v), None => std::env::remove_var("HUAWEICLOUD_SDK_AK") };
    match old_sk { Some(v) => std::env::set_var("HUAWEICLOUD_SDK_SK", v), None => std::env::remove_var("HUAWEICLOUD_SDK_SK") };
}

#[test]
fn virtual_addressing_builds_bucket_prefixed_host() {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_addressing_style(AddressingStyle::Virtual);
    let url = build_object_url(&cfg, "my-bucket", "photos/cat.jpg").unwrap();
    assert_eq!(
        url.as_str(),
        "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg"
    );
}

#[test]
fn path_addressing_builds_path_style_url() {
    let cfg = ClientConfig::for_region("cn-north-4")
        .with_addressing_style(AddressingStyle::Path);
    let url = build_object_url(&cfg, "my-bucket", "photos/cat.jpg").unwrap();
    assert_eq!(
        url.as_str(),
        "https://obs.cn-north-4.myhuaweicloud.com/my-bucket/photos/cat.jpg"
    );
}

#[test]
fn auto_addressing_falls_back_to_path_when_bucket_has_dot() {
    let cfg = ClientConfig::for_region("cn-north-4");
    let url = build_object_url(&cfg, "bucket.with.dots", "k").unwrap();
    assert!(url.host_str().unwrap().starts_with("obs."));
    assert!(url.path().starts_with("/bucket.with.dots/"));
}

#[test]
fn signing_version_defaults_to_v4() {
    let cfg = ClientConfig::for_region("cn-north-4");
    assert!(matches!(cfg.signing_version, SigningVersion::V4));
}
