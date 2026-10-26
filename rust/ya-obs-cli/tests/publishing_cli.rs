use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
fn cli(server: &MockServer) -> assert_cmd::Command {
    cli_at(&server.uri())
}
fn cli_at(endpoint: &str) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("ya-obs").unwrap();
    cmd.env("YA_OBS_CONFIG", "/nonexistent/ya-obs-test-config.toml")
        .env("HUAWEICLOUD_SDK_AK", "test-ak")
        .env("HUAWEICLOUD_SDK_SK", "test-secret-never-log")
        .args([
            "--endpoint",
            endpoint,
            "--region",
            "cn-southwest-2",
            "--output",
            "json",
            "--quiet",
        ]);
    cmd
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_errors_have_stable_conflict_permission_and_transport_codes() {
    let file = tempfile::NamedTempFile::new().unwrap();
    for (status, code, exit) in [
        (409, "Conflict", 7),
        (412, "HttpError", 7),
        (403, "AccessDenied", 5),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
        let output = cli(&server)
            .args([
                "cp",
                file.path().to_str().unwrap(),
                "obs://test.bucket/key",
                "--acl",
                "public-read",
            ])
            .assert()
            .code(exit)
            .get_output()
            .clone();
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code);
        assert_eq!(error["error"]["status"], status);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("test-secret"));
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut cmd = cli_at(&endpoint);
    let output = cmd
        .args(["cp", file.path().to_str().unwrap(), "obs://test.bucket/key"])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "Transport");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_modes_fail_before_network_or_stdin_read() {
    let server = MockServer::start().await;
    for args in [
        vec!["cp", "-", "obs://test.bucket/key", "--no-clobber"],
        vec![
            "cp",
            "dir",
            "obs://test.bucket/key",
            "-r",
            "--acl",
            "public-read",
        ],
        vec![
            "cp",
            "obs://test.bucket/key",
            "local",
            "--content-type",
            "application/zip",
        ],
    ] {
        let output = cli(&server)
            .args(args)
            .assert()
            .code(8)
            .get_output()
            .clone();
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "Unsupported");
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(100 * 1024 * 1024).unwrap();
    cli(&server)
        .args([
            "cp",
            file.path().to_str().unwrap(),
            "obs://test.bucket/key",
            "--no-clobber",
        ])
        .assert()
        .code(8);
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stat_exposes_publishing_properties_and_acl_is_object_only() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/test.bucket/key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-obs-meta-sha256", "archive-hash")
                .insert_header("cache-control", "immutable")
                .insert_header("content-type", "application/zip")
                .insert_header("content-disposition", "attachment"),
        )
        .mount(&server)
        .await;
    let output = cli(&server)
        .args(["stat", "obs://test.bucket/key"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stat: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stat["metadata"]["sha256"], "archive-hash");
    assert_eq!(stat["content_type"], "application/zip");
    assert_eq!(stat["cache_control"], "immutable");
    assert_eq!(stat["content_disposition"], "attachment");
    Mock::given(method("PUT"))
        .and(path("/test.bucket/key"))
        .and(|req: &wiremock::Request| req.url.query() == Some("acl"))
        .and(header("x-obs-acl", "public-read"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    cli(&server)
        .args([
            "acl",
            "set",
            "obs://test.bucket/key",
            "--acl",
            "public-read",
        ])
        .assert()
        .success();
    cli(&server)
        .args(["acl", "set", "obs://test.bucket", "--acl", "public-read"])
        .assert()
        .code(3);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_copy_keeps_single_put_behavior_and_json_summary() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let file = tempfile::NamedTempFile::new().unwrap();
    let output = cli(&server)
        .args(["cp", file.path().to_str().unwrap(), "obs://test.bucket/key"])
        .assert()
        .success()
        .get_output()
        .clone();
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["ok"], true);
    assert_eq!(summary["multipart"], false);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("x-obs-acl"));
    assert!(!requests[0].headers.contains_key("x-obs-forbid-overwrite"));
}
