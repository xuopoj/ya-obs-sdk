import json
import pathlib
from ya_obs._xml import (
    parse_list_objects,
    parse_initiate_multipart,
    parse_error_response,
    serialize_complete_multipart,
)

VECTORS = pathlib.Path(__file__).parent.parent.parent / "test-vectors" / "xml"

def load(name):
    return json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))

def test_parse_list_objects():
    v = load("list_objects_response")
    result = parse_list_objects(v["input"]["xml"])
    assert result["name"] == v["expected"]["name"]
    assert result["is_truncated"] == v["expected"]["is_truncated"]
    assert result["next_marker"] == v["expected"]["next_marker"]
    assert len(result["objects"]) == 2
    assert result["objects"][0]["key"] == "photo.jpg"
    assert result["objects"][0]["size"] == 12345
    assert result["objects"][1]["key"] == "video.mp4"

def test_parse_initiate_multipart():
    v = load("initiate_multipart_response")
    result = parse_initiate_multipart(v["input"]["xml"])
    assert result == v["expected"]

def test_parse_error_response():
    v = load("error_response")
    result = parse_error_response(v["input"]["xml"])
    assert result == v["expected"]

def test_serialize_complete_multipart():
    v = load("complete_multipart_request")
    parts = [(p["part_number"], p["etag"]) for p in v["input"]["parts"]]
    xml = serialize_complete_multipart(parts)
    assert xml == v["expected"]["xml"]
