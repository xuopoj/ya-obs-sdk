use std::io::Write;
use ya_obs_cli::config::{load, pick_first};

fn write_tmp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn missing_file_returns_empty_config() {
    let path = std::path::PathBuf::from("/no/such/path/never/exists.toml");
    let cfg = load(&path, None).unwrap();
    assert_eq!(cfg.profile.region, None);
    assert_eq!(cfg.profile.endpoint, None);
    assert!(cfg.warnings.is_empty());
}

#[test]
fn default_section_loads() {
    let f = write_tmp(
        r#"
[default]
region = "cn-north-4"
signing_version = "v4"
"#,
    );
    let cfg = load(f.path(), None).unwrap();
    assert_eq!(cfg.profile.region.as_deref(), Some("cn-north-4"));
    assert_eq!(cfg.profile.signing_version.as_deref(), Some("v4"));
}

#[test]
fn named_profile_loads() {
    let f = write_tmp(
        r#"
[default]
region = "cn-north-4"

[profiles.nisco]
region = "cn-global-1"
endpoint = "https://obsv3.cloud.nisco.cn"
"#,
    );
    let cfg = load(f.path(), Some("nisco")).unwrap();
    assert_eq!(cfg.profile.region.as_deref(), Some("cn-global-1"));
    assert_eq!(
        cfg.profile.endpoint.as_deref(),
        Some("https://obsv3.cloud.nisco.cn")
    );
}

#[test]
fn unknown_profile_errors() {
    let f = write_tmp(
        r#"[default]
region = "x"
"#,
    );
    let err = load(f.path(), Some("nope")).unwrap_err();
    assert!(err.contains("profile 'nope' not found"));
}

#[test]
fn pick_first_prefers_cli_then_env_then_file() {
    assert_eq!(
        pick_first([Some("cli".into()), Some("env".into()), Some("file".into())]).as_deref(),
        Some("cli")
    );
    assert_eq!(
        pick_first([None, Some("env".into()), Some("file".into())]).as_deref(),
        Some("env")
    );
    assert_eq!(
        pick_first([None, None, Some("file".into())]).as_deref(),
        Some("file")
    );
    assert_eq!(pick_first([None, None, None]), None);
}

#[cfg(unix)]
#[test]
fn warns_on_loose_perms_when_secrets_present() {
    use std::os::unix::fs::PermissionsExt;
    let f = write_tmp(
        r#"
[default]
access_key = "AK"
secret_key = "SK"
"#,
    );
    std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
    let cfg = load(f.path(), None).unwrap();
    assert!(
        !cfg.warnings.is_empty(),
        "expected a perms warning, got none"
    );
    assert!(cfg.warnings[0].contains("readable by other users"));
}

#[cfg(unix)]
#[test]
fn no_warning_when_perms_are_tight() {
    use std::os::unix::fs::PermissionsExt;
    let f = write_tmp(
        r#"
[default]
access_key = "AK"
secret_key = "SK"
"#,
    );
    std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    let cfg = load(f.path(), None).unwrap();
    assert!(
        cfg.warnings.is_empty(),
        "unexpected warnings: {:?}",
        cfg.warnings
    );
}
