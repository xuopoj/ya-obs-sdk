use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_prints_subcommands() {
    Command::cargo_bin("ya-obs").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("ls"))
        .stdout(contains("cp"))
        .stdout(contains("rm"))
        .stdout(contains("cat"))
        .stdout(contains("presign"));
}

#[test]
fn obs_uri_parses_bucket_and_key() {
    use ya_obs_cli::obs_uri::ObsUri;
    let u: ObsUri = "obs://my-bucket/path/to/file.txt".parse().unwrap();
    assert_eq!(u.bucket, "my-bucket");
    assert_eq!(u.key, "path/to/file.txt");
}

#[test]
fn obs_uri_rejects_non_obs_scheme() {
    use ya_obs_cli::obs_uri::ObsUri;
    let r: Result<ObsUri, _> = "s3://b/k".parse();
    assert!(r.is_err());
}

#[test]
fn obs_uri_accepts_bucket_only() {
    use ya_obs_cli::obs_uri::ObsUri;
    let u: ObsUri = "obs://my-bucket".parse().unwrap();
    assert_eq!(u.bucket, "my-bucket");
    assert_eq!(u.key, "");
}
