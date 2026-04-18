import json
import pathlib
from ya_obs._signer_v4 import SignerV4, canonical_request, string_to_sign
from ya_obs._signer_v2 import SignerV2, build_string_to_sign, build_canonicalized_resource

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "auth"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_v4_canonical_request_vector():
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

def test_v2_string_to_sign_basic_vector():
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

def test_v2_string_to_sign_content_md5_vector():
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

def test_v2_presign_string_to_sign_vector():
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
