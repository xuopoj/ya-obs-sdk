import json
import pathlib
from ya_obs._signer_v4 import SignerV4, canonical_request, string_to_sign
from ya_obs._models import Request

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_canonical_request_structure():
    v = load("v4_header_basic")
    i = v["input"]
    cr = canonical_request(
        method=i["method"],
        path="/photos/cat.jpg",
        query_string="",
        headers=i["headers"],
        signed_headers="host;x-amz-content-sha256;x-amz-date",
        body_sha256=i["body_sha256"],
    )
    assert cr == v["expected"]["canonical_request"]

def test_string_to_sign_structure():
    v = load("v4_header_basic")
    i = v["input"]
    cr = v["expected"]["canonical_request"]
    sts = string_to_sign(
        datetime_str=i["headers"]["x-amz-date"],
        date_str=i["date"],
        region=i["region"],
        service=i["service"],
        canonical_request=cr,
    )
    assert sts.startswith(v["expected"]["string_to_sign"])

def test_sign_request_adds_authorization():
    signer = SignerV4(
        access_key="AKIAIOSFODNN7EXAMPLE",
        secret_key="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        region="cn-north-4",
    )
    req = Request(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/photos/cat.jpg",
        headers={},
    )
    signed = signer.sign(req)
    assert "Authorization" in signed.headers
    assert signed.headers["Authorization"].startswith("AWS4-HMAC-SHA256 ")

def test_presign_url_contains_required_params():
    v = load("v4_presign_basic")
    signer = SignerV4(
        access_key=v["input"]["access_key"],
        secret_key=v["input"]["secret_key"],
        region=v["input"]["region"],
    )
    url = signer.presign(
        method="GET",
        url=v["input"]["url"],
        expires=v["input"]["expires"],
    )
    for param in v["expected"]["required_params"]:
        assert param in url, f"Missing {param} in presigned URL"
    assert "AWS4-HMAC-SHA256" in url
    assert "3600" in url
