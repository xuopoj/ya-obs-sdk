import json
import pathlib
import pytest
from ya_obs._errors import make_error, NoSuchKey, NoSuchBucket, AccessDenied
from ya_obs._xml import parse_error_response

VECTORS = pathlib.Path(__file__).parent.parent.parent.parent / "test-vectors" / "errors"
CLASS_MAP = {"NoSuchKey": NoSuchKey, "NoSuchBucket": NoSuchBucket, "AccessDenied": AccessDenied}

@pytest.mark.parametrize("name", ["no_such_key", "no_such_bucket", "access_denied"])
def test_error_vector(name):
    v = json.loads((VECTORS / f"{name}.json").read_text(encoding="utf-8"))
    parsed = parse_error_response(v["input"]["xml"])
    err = make_error(
        code=parsed["code"],
        message=parsed["message"],
        status=v["input"]["status"],
        request_id=parsed["request_id"],
        host_id=parsed["host_id"],
    )
    expected_class = CLASS_MAP[v["expected"]["exception_class"]]
    assert isinstance(err, expected_class)
    assert err.code == v["expected"]["code"]
    assert err.status == v["expected"]["status"]
