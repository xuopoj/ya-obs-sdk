import json
import pathlib
from urllib.parse import urlparse, parse_qs
from ya_obs._signer_v4 import SignerV4, canonical_request, string_to_sign, _encode_canonical_query
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

def test_canonical_query_sorts_and_encodes():
    # keys sorted, spaces as %20 (not +), empty value preserved as "k="
    q = _encode_canonical_query("", {"prefix": "a/b c", "max-keys": "5", "uploads": ""})
    assert q == "max-keys=5&prefix=a%2Fb%20c&uploads="


def test_canonical_query_merges_url_query_and_params():
    q = _encode_canonical_query("uploadId=abc==", {"partNumber": "3"})
    # '=' in the value must be percent-encoded
    assert q == "partNumber=3&uploadId=abc%3D%3D"


def test_canonical_query_empty_when_nothing():
    assert _encode_canonical_query("", None) == ""
    assert _encode_canonical_query("", {}) == ""


def test_sign_bakes_params_into_url_and_clears_params():
    signer = SignerV4(access_key="AK", secret_key="SK", region="cn-global-1")
    req = Request(
        method="GET",
        url="https://bucket.example/",
        params={"max-keys": "5", "prefix": "a/b"},
    )
    signed = signer.sign(req)
    assert signed.params is None
    parsed = urlparse(signed.url)
    qs = parse_qs(parsed.query)
    assert qs == {"max-keys": ["5"], "prefix": ["a/b"]}


def test_sign_list_vs_put_produce_different_signatures():
    # If params weren't signed, list_objects and put_object to the same URL
    # would share a signature. After the fix they must differ.
    signer = SignerV4(access_key="AK", secret_key="SK", region="cn-global-1")
    url = "https://bucket.example/"
    put_sig = signer.sign(Request(method="GET", url=url)).headers["Authorization"]
    list_sig = signer.sign(
        Request(method="GET", url=url, params={"max-keys": "1"})
    ).headers["Authorization"]
    assert put_sig != list_sig


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
