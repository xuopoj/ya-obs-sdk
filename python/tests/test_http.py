import ssl

import httpx
import pytest
from pytest_httpx import HTTPXMock
from ya_obs._http import HttpClient, AsyncHttpClient
from ya_obs._signer_v4 import SignerV4
from ya_obs._models import Request, RetryPolicy, Timeout
from ya_obs._errors import NoSuchKey, ServerError
from ya_obs.client import Client, AsyncClient

@pytest.fixture
def signer():
    return SignerV4(access_key="TESTKEY", secret_key="TESTSECRET", region="cn-north-4")

@pytest.fixture
def client(signer):
    return HttpClient(
        signer=signer,
        timeout=Timeout(connect=5, read=10),
        retry_policy=RetryPolicy(max_attempts=1),
    )

def test_successful_get(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key.txt",
        status_code=200,
        content=b"hello",
        headers={"x-obs-request-id": "srv-req-123", "ETag": '"abc"'},
    )
    req = Request(method="GET", url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/key.txt")
    resp = client.send(req)
    assert resp.status_code == 200
    assert resp.headers["x-obs-request-id"] == "srv-req-123"

def test_404_raises_no_such_key(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/missing.txt",
        status_code=404,
        content=b'<?xml version="1.0"?><Error><Code>NoSuchKey</Code><Message>Not found</Message><RequestId>r1</RequestId><HostId>h1</HostId></Error>',
        headers={"Content-Type": "application/xml"},
    )
    req = Request(method="GET", url="https://my-bucket.obs.cn-north-4.myhuaweicloud.com/missing.txt")
    with pytest.raises(NoSuchKey) as exc_info:
        client.send(req)
    assert exc_info.value.status == 404
    assert exc_info.value.code == "NoSuchKey"

def test_successful_send_closes_response_connection(httpx_mock: HTTPXMock, client):
    """Regression: non-streaming send must close the httpx.Response so the
    connection returns to the pool. Previously it leaked, causing one new
    TCP+TLS handshake per request (see the multipart stall bug)."""
    httpx_mock.add_response(
        method="PUT",
        url="https://b.obs.cn-north-4.myhuaweicloud.com/k",
        status_code=200,
        content=b"",
    )
    resp = client.send(Request(method="PUT", url="https://b.obs.cn-north-4.myhuaweicloud.com/k", body=b"x"))
    # pytest_httpx exposes the underlying httpx.Response via transport; we can
    # check is_closed on our wrapped response
    assert resp._response.is_closed, "response was not closed after non-streaming send"


def test_streaming_send_leaves_response_open(httpx_mock: HTTPXMock, client):
    """Streaming callers take ownership of closing — the send itself must NOT
    close or the caller will get EOF mid-stream."""
    httpx_mock.add_response(
        method="GET",
        url="https://b.obs.cn-north-4.myhuaweicloud.com/k",
        status_code=200,
        content=b"streamed body",
    )
    resp = client.send(Request(method="GET", url="https://b.obs.cn-north-4.myhuaweicloud.com/k"), stream=True)
    assert not resp._response.is_closed
    # Draining via read() should close it.
    data = resp.read()
    assert data == b"streamed body"
    assert resp._response.is_closed


def test_request_gets_client_id_header(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        url="https://bucket.obs.cn-north-4.myhuaweicloud.com/k",
        status_code=200,
        content=b"",
    )
    req = Request(method="GET", url="https://bucket.obs.cn-north-4.myhuaweicloud.com/k")
    client.send(req)
    sent = httpx_mock.get_requests()
    assert any("x-ya-obs-client-id" in str(r.headers).lower() for r in sent)


def _capture_httpx(monkeypatch, attr):
    captured = {}

    class _FakeHttpxClient:
        def __init__(self, *args, **kwargs):
            captured.update(kwargs)
        def close(self):
            pass
        async def aclose(self):
            pass

    monkeypatch.setattr(httpx, attr, _FakeHttpxClient)
    return captured


def test_http_client_defaults_verify_true(monkeypatch, signer):
    captured = _capture_httpx(monkeypatch, "Client")
    HttpClient(signer=signer, timeout=Timeout(), retry_policy=RetryPolicy())
    assert captured["verify"] is True


def test_http_client_passes_verify_false(monkeypatch, signer):
    captured = _capture_httpx(monkeypatch, "Client")
    HttpClient(signer=signer, timeout=Timeout(), retry_policy=RetryPolicy(), verify=False)
    assert captured["verify"] is False


def test_http_client_passes_ca_bundle_path(monkeypatch, signer, tmp_path):
    ca = tmp_path / "ca.pem"
    ca.write_text("-----BEGIN CERTIFICATE-----\n")
    captured = _capture_httpx(monkeypatch, "Client")
    HttpClient(signer=signer, timeout=Timeout(), retry_policy=RetryPolicy(), verify=str(ca))
    assert captured["verify"] == str(ca)


def test_http_client_passes_ssl_context(monkeypatch, signer):
    ctx = ssl.create_default_context()
    captured = _capture_httpx(monkeypatch, "Client")
    HttpClient(signer=signer, timeout=Timeout(), retry_policy=RetryPolicy(), verify=ctx)
    assert captured["verify"] is ctx


def test_async_http_client_passes_verify(monkeypatch, signer):
    captured = _capture_httpx(monkeypatch, "AsyncClient")
    AsyncHttpClient(signer=signer, timeout=Timeout(), retry_policy=RetryPolicy(), verify=False)
    assert captured["verify"] is False


def test_client_plumbs_verify_to_httpx(monkeypatch):
    captured = _capture_httpx(monkeypatch, "Client")
    Client(
        access_key="AK", secret_key="SK", region="cn-north-4",
        verify=False,
    )
    assert captured["verify"] is False


def test_async_client_plumbs_verify_to_httpx(monkeypatch):
    captured = _capture_httpx(monkeypatch, "AsyncClient")
    AsyncClient(
        access_key="AK", secret_key="SK", region="cn-north-4",
        verify=False,
    )
    assert captured["verify"] is False


def test_client_verify_defaults_to_true(monkeypatch):
    captured = _capture_httpx(monkeypatch, "Client")
    Client(access_key="AK", secret_key="SK", region="cn-north-4")
    assert captured["verify"] is True
