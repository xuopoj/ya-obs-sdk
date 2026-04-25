import os
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


def test_get_object_save_to_with_progress(httpx_mock: HTTPXMock, client, tmp_path):
    payload = b"download progress payload"
    httpx_mock.add_response(
        method="GET", status_code=200, content=payload,
        headers={"ETag": '"e"', "Content-Length": str(len(payload)),
                 "Last-Modified": "Mon, 15 Jan 2024 10:30:00 GMT",
                 "x-obs-request-id": "r"},
    )
    events = []
    resp = client.get_object("my-bucket", "file.txt")
    dest = tmp_path / "out.bin"
    resp.body.save_to(dest, on_progress=events.append)
    assert dest.read_bytes() == payload
    assert events[-1].bytes_transferred == len(payload)
    assert events[-1].total_bytes == len(payload)


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


@pytest.mark.skipif(not hasattr(os, "pread"), reason="pread not available")
def test_put_object_path_large_uses_streaming_multipart(httpx_mock: HTTPXMock, client, tmp_path):
    threshold = 2048
    part_size = 1024
    file_size = part_size * 3 + 200  # 4 parts, well above threshold

    file_path = tmp_path / "big.bin"
    with open(file_path, "wb") as f:
        f.write(b"A" * part_size)
        f.write(b"B" * part_size)
        f.write(b"C" * part_size)
        f.write(b"D" * 200)

    initiate_xml = (
        b'<?xml version="1.0" encoding="UTF-8"?>'
        b'<InitiateMultipartUploadResult xmlns="http://obs.myhwclouds.com/doc/2015-06-30/">'
        b'<UploadId>up-100</UploadId></InitiateMultipartUploadResult>'
    )
    httpx_mock.add_response(
        method="POST",
        status_code=200,
        content=initiate_xml,
        headers={"Content-Type": "application/xml", "x-obs-request-id": "init"},
    )
    for i in range(1, 5):
        httpx_mock.add_response(
            method="PUT",
            status_code=200,
            headers={"ETag": f'"part-{i}"', "x-obs-request-id": f"p{i}"},
        )
    httpx_mock.add_response(
        method="POST",
        status_code=200,
        content=b'<?xml version="1.0"?><CompleteMultipartUploadResult><ETag>"final"</ETag></CompleteMultipartUploadResult>',
        headers={"ETag": '"final-etag"', "x-obs-request-id": "done"},
    )

    from pathlib import Path as _P
    resp = client.put_object(
        "my-bucket", "big.bin", _P(file_path),
        multipart_threshold=threshold,
        part_size=part_size,
        concurrency=2,
    )

    assert resp.etag == '"final-etag"'
    sent = httpx_mock.get_requests()
    part_puts = [r for r in sent if r.method == "PUT" and b"partNumber" in r.url.query]
    assert len(part_puts) == 4
    total_uploaded = sum(len(r.content) for r in part_puts)
    assert total_uploaded == file_size
    sizes = sorted(len(r.content) for r in part_puts)
    assert sizes == [200, part_size, part_size, part_size]


def test_put_object_small_fires_progress_once(httpx_mock: HTTPXMock, client):
    httpx_mock.add_response(
        method="PUT", status_code=200,
        headers={"ETag": '"e"', "x-obs-request-id": "r"},
    )
    events = []
    body = b"hello world"
    client.put_object("b", "k", body, on_progress=events.append)
    assert len(events) == 1
    assert events[0].bytes_transferred == len(body)
    assert events[0].total_bytes == len(body)
    assert events[0].part_number is None
