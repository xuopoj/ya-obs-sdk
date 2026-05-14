use ya_obs_cli::commands::init;

#[test]
fn writes_template_to_new_path() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("subdir").join("config.toml");

    init::run(Some(target.to_str().unwrap()), false, None).unwrap();

    let body = std::fs::read_to_string(&target).unwrap();
    assert!(body.contains("[default]"));
    assert!(body.contains("# region ="));
    assert!(body.contains("[profiles.nisco]"));
}

#[test]
fn refuses_to_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("config.toml");
    std::fs::write(&target, "existing user content").unwrap();

    let err = init::run(Some(target.to_str().unwrap()), false, None).unwrap_err();
    assert!(err.to_string().contains("--force"));
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "existing user content"
    );
}

#[test]
fn force_overwrites_existing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("config.toml");
    std::fs::write(&target, "existing").unwrap();

    init::run(Some(target.to_str().unwrap()), true, None).unwrap();

    let body = std::fs::read_to_string(&target).unwrap();
    assert!(body.contains("[default]"));
}

#[cfg(unix)]
#[test]
fn sets_mode_600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("config.toml");

    init::run(Some(target.to_str().unwrap()), false, None).unwrap();

    let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
}

#[test]
fn errors_when_no_path_and_no_fallback() {
    let err = init::run(None, false, None).unwrap_err();
    assert!(err
        .to_string()
        .contains("cannot determine default config path"));
}
