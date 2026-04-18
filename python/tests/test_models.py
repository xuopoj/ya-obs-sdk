from ya_obs._models import Request, Timeout, RetryPolicy, RetryEvent
from ya_obs._errors import ObsError, ClientError, ServerError, NoSuchKey, NoSuchBucket, AccessDenied

def test_request_defaults():
    r = Request(method="GET", url="https://example.com/bucket/key")
    assert r.headers == {}
    assert r.body is None
    assert r.params == {}

def test_timeout_defaults():
    t = Timeout()
    assert t.connect == 10.0
    assert t.read == 60.0
    assert t.total is None

def test_retry_policy_defaults():
    rp = RetryPolicy()
    assert rp.max_attempts == 3
    assert rp.base_delay == 0.5
    assert rp.max_delay == 30.0
    assert rp.jitter is True

def test_error_hierarchy():
    assert issubclass(ClientError, ObsError)
    assert issubclass(ServerError, ObsError)
    assert issubclass(NoSuchKey, ClientError)
    assert issubclass(NoSuchBucket, ClientError)
    assert issubclass(AccessDenied, ClientError)

def test_obs_error_fields():
    e = NoSuchKey(
        code="NoSuchKey",
        message="The specified key does not exist.",
        status=404,
        request_id="ABC123",
        host_id="HOST456",
        client_request_id="cli-uuid",
    )
    assert e.code == "NoSuchKey"
    assert e.status == 404
    assert str(e) == "NoSuchKey: The specified key does not exist. (request_id=ABC123)"
