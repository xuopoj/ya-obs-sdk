import json
import pathlib
from ya_obs._signer_v2 import SignerV2, build_string_to_sign, build_canonicalized_resource
from ya_obs._models import Request

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_string_to_sign_basic():
    v = load("v2_header_basic")
    i = v["input"]
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=f"/{i['bucket']}/{i['key']}",
    )
    assert sts == v["expected"]["string_to_sign"]

def test_string_to_sign_with_content_md5():
    v = load("v2_header_content_md5")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5=i["content_md5"],
        content_type=i["content_type"],
        date=i["date"],
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]

def test_sign_request_adds_authorization():
    signer = SignerV2(
        access_key="AKIAIOSFODNN7EXAMPLE",
        secret_key="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    )
    req = Request(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
        headers={"Date": "Tue, 15 Jan 2024 10:30:00 GMT"},
    )
    signed = signer.sign(req)
    assert "Authorization" in signed.headers
    assert signed.headers["Authorization"].startswith("OBS AKIAIOSFODNN7EXAMPLE:")

def test_presign_string_to_sign():
    v = load("v2_presign_basic")
    i = v["input"]
    resource = build_canonicalized_resource(bucket=i["bucket"], key=i["key"])
    sts = build_string_to_sign(
        method=i["method"],
        content_md5="",
        content_type="",
        date=str(i["expires_unix"]),
        obs_headers=i["obs_headers"],
        canonicalized_resource=resource,
    )
    assert sts == v["expected"]["string_to_sign"]

def test_v2_sub_resources_included_in_canonicalized_resource():
    resource = build_canonicalized_resource(
        bucket="b", key="", sub_resources={"uploads": ""},
    )
    assert resource == "/b/?uploads"


def test_v2_sub_resources_filter_out_non_sub_resources():
    # "max-keys" is not a sub-resource in the V2 list — it must NOT appear
    resource = build_canonicalized_resource(
        bucket="b", key="",
        sub_resources={"uploads": "", "max-keys": "5"},
    )
    assert resource == "/b/?uploads"
    assert "max-keys" not in resource


def test_v2_sign_bakes_params_into_url():
    signer = SignerV2(access_key="AK", secret_key="SK")
    req = Request(
        method="POST",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key",
        params={"uploads": ""},
    )
    signed = signer.sign(req)
    assert signed.params is None
    assert signed.url.endswith("?uploads")


def test_v2_sign_differs_with_and_without_params():
    signer = SignerV2(access_key="AK", secret_key="SK")
    url = "https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key"
    date = "Tue, 15 Jan 2024 10:30:00 GMT"
    a = signer.sign(Request(method="POST", url=url, headers={"Date": date})).headers["Authorization"]
    b = signer.sign(Request(method="POST", url=url, headers={"Date": date}, params={"uploads": ""})).headers["Authorization"]
    assert a != b


def test_presign_url_contains_required_params():
    v = load("v2_presign_basic")
    i = v["input"]
    signer = SignerV2(access_key=i["access_key"], secret_key=i["secret_key"])
    url = signer.presign(
        method=i["method"],
        url=f"https://obs.cn-north-4.myhuaweicloud.com/{i['bucket']}/{i['key']}",
        expires_unix=i["expires_unix"],
        bucket=i["bucket"],
        key=i["key"],
    )
    for param in v["expected"]["required_params"]:
        assert param in url
