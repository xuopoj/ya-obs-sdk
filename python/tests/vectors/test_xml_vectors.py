import json
import pathlib
from ya_obs._xml import (
    parse_list_objects,
    parse_initiate_multipart,
    parse_error_response,
    serialize_complete_multipart,
)

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "xml"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_list_objects_vector():
    v = load("list_objects_response")
    result = parse_list_objects(v["input"]["xml"])
    assert result["name"] == v["expected"]["name"]
    assert result["is_truncated"] == v["expected"]["is_truncated"]
    assert len(result["objects"]) == len(v["expected"]["objects"])
    for got, want in zip(result["objects"], v["expected"]["objects"]):
        assert got["key"] == want["key"]
        assert got["size"] == want["size"]
        assert got["etag"] == want["etag"]

def test_initiate_multipart_vector():
    v = load("initiate_multipart_response")
    assert parse_initiate_multipart(v["input"]["xml"]) == v["expected"]

def test_error_response_vector():
    v = load("error_response")
    assert parse_error_response(v["input"]["xml"]) == v["expected"]

def test_complete_multipart_vector():
    v = load("complete_multipart_request")
    parts = [(p["part_number"], p["etag"]) for p in v["input"]["parts"]]
    assert serialize_complete_multipart(parts) == v["expected"]["xml"]
