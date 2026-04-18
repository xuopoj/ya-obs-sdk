import pytest
from pytest_httpx import HTTPXMock
from ya_obs.client import AsyncClient

@pytest.fixture
def async_client():
    return AsyncClient(access_key="TESTKEY", secret_key="TESTSECRET", region="cn-north-4")

@pytest.mark.asyncio
async def test_async_put_object(httpx_mock: HTTPXMock, async_client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-async"', "x-obs-request-id": "req-async-1"},
    )
    resp = await async_client.put_object("my-bucket", "async-file.txt", b"async content")
    assert resp.etag == '"etag-async"'
    assert resp.request_id == "req-async-1"
    await async_client.close()

@pytest.mark.asyncio
async def test_async_get_object(httpx_mock: HTTPXMock, async_client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=b"async file contents",
        headers={
            "ETag": '"def"',
            "Content-Type": "text/plain",
            "Content-Length": "19",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-async-2",
        },
    )
    resp = await async_client.get_object("my-bucket", "async-file.txt")
    data = await resp.body.read()
    assert data == b"async file contents"
    await async_client.close()

@pytest.mark.asyncio
async def test_async_context_manager(httpx_mock: HTTPXMock):
    httpx_mock.add_response(
        method="DELETE",
        status_code=204,
        headers={"x-obs-request-id": "req-async-3"},
    )
    async with AsyncClient(access_key="K", secret_key="S", region="cn-north-4") as client:
        resp = await client.delete_object("my-bucket", "file.txt")
        assert resp.request_id == "req-async-3"
