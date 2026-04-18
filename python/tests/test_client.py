import pytest
from pytest_httpx import HTTPXMock
from ya_obs.client import Client
from ya_obs._errors import NoSuchKey

@pytest.fixture
def client():
    return Client(access_key="TESTKEY", secret_key="TESTSECRET", region="cn-north-4")

LIST_XML = b"""<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">
  <Name>my-bucket</Name><Prefix></Prefix><MaxKeys>1000</MaxKeys>
  <IsTruncated>false</IsTruncated>
  <Contents>
    <Key>file.txt</Key>
    <LastModified>2024-01-15T10:30:00.000Z</LastModified>
    <ETag>"abc123"</ETag>
    <Size>42</Size>
  </Contents>
</ListBucketResult>"""

def test_put_object_small(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-abc"', "x-obs-request-id": "req-1"},
    )
    resp = client.put_object("my-bucket", "file.txt", b"hello world")
    assert resp.etag == '"etag-abc"'
    assert resp.request_id == "req-1"

def test_put_object_with_metadata(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="PUT",
        status_code=200,
        headers={"ETag": '"etag-xyz"', "x-obs-request-id": "req-2"},
    )
    resp = client.put_object(
        "my-bucket", "file.txt", b"data",
        content_type="text/plain",
        metadata={"author": "sean", "project": "obs"},
    )
    sent = httpx_mock.get_requests()
    assert sent[0].headers.get("content-type") == "text/plain"
    assert sent[0].headers.get("x-obs-meta-author") == "sean"

def test_get_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=b"file contents",
        headers={
            "ETag": '"abc"',
            "Content-Type": "text/plain",
            "Content-Length": "13",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-3",
        },
    )
    resp = client.get_object("my-bucket", "file.txt")
    assert resp.etag == '"abc"'
    assert resp.content_type == "text/plain"
    assert resp.body.read() == b"file contents"

def test_get_object_range(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=206,
        content=b"llo",
        headers={"ETag": '"abc"', "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT", "x-obs-request-id": "r"},
    )
    client.get_object("my-bucket", "file.txt", range=(2, 4))
    sent = httpx_mock.get_requests()
    assert sent[0].headers.get("range") == "bytes=2-4"

def test_head_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="HEAD",
        status_code=200,
        headers={
            "ETag": '"abc"',
            "Content-Type": "image/jpeg",
            "Content-Length": "1234",
            "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
            "x-obs-request-id": "req-4",
            "x-obs-meta-author": "sean",
        },
    )
    resp = client.head_object("my-bucket", "photo.jpg")
    assert resp.etag == '"abc"'
    assert resp.metadata == {"author": "sean"}

def test_delete_object(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(method="DELETE", status_code=204,
        headers={"x-obs-request-id": "req-5"})
    resp = client.delete_object("my-bucket", "file.txt")
    assert resp.request_id == "req-5"

def test_list_objects_page(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=LIST_XML,
        headers={"Content-Type": "application/xml", "x-obs-request-id": "req-6"},
    )
    page = client.list_objects_page("my-bucket")
    assert len(page.objects) == 1
    assert page.objects[0].key == "file.txt"
    assert page.objects[0].size == 42
    assert page.is_truncated is False

def test_list_objects_iterator(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="GET",
        status_code=200,
        content=LIST_XML,
        headers={"Content-Type": "application/xml", "x-obs-request-id": "req-7"},
    )
    objects = list(client.list_objects("my-bucket"))
    assert len(objects) == 1
    assert objects[0].key == "file.txt"
