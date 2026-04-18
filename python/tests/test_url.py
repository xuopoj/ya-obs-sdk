from ya_obs._url import build_url, is_obs_domain, is_dns_safe_bucket

def test_virtual_hosted_obs_domain():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my-bucket",
        key="photos/cat.jpg",
        addressing_style="auto",
    )
    assert url == "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg"

def test_path_style_forced():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my-bucket",
        key="photos/cat.jpg",
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/my-bucket/photos/cat.jpg"

def test_auto_path_style_for_dotted_bucket():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="my.bucket.with.dots",
        key="key.txt",
        addressing_style="auto",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/my.bucket.with.dots/key.txt"

def test_auto_path_style_for_custom_endpoint():
    url = build_url(
        endpoint="http://localhost:9000",
        bucket="testbucket",
        key="file.bin",
        addressing_style="auto",
    )
    assert url == "http://localhost:9000/testbucket/file.bin"

def test_key_encoding():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="mybucket",
        key="path/to/my file & stuff.txt",
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/mybucket/path/to/my%20file%20%26%20stuff.txt"

def test_no_key():
    url = build_url(
        endpoint="https://obs.cn-north-4.myhuaweicloud.com",
        bucket="mybucket",
        key=None,
        addressing_style="path",
    )
    assert url == "https://obs.cn-north-4.myhuaweicloud.com/mybucket/"

def test_is_obs_domain():
    assert is_obs_domain("https://obs.cn-north-4.myhuaweicloud.com") is True
    assert is_obs_domain("https://obs.eu-west-0.myhuaweicloud.eu") is True
    assert is_obs_domain("http://localhost:9000") is False
    assert is_obs_domain("https://minio.example.com") is False

def test_is_dns_safe_bucket():
    assert is_dns_safe_bucket("my-bucket") is True
    assert is_dns_safe_bucket("my.bucket") is False
    assert is_dns_safe_bucket("MY-BUCKET") is False
    assert is_dns_safe_bucket("a" * 63) is True
    assert is_dns_safe_bucket("a" * 64) is False
