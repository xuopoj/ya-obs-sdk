import pytest
from ya_obs._streaming import StreamingBody

def _make_body(data: bytes) -> StreamingBody:
    def _iter():
        chunk_size = 4
        for i in range(0, len(data), chunk_size):
            yield data[i:i + chunk_size]
    return StreamingBody(iterator=_iter())

def test_read_buffers_all():
    body = _make_body(b"hello world")
    assert body.read() == b"hello world"

def test_iter_bytes():
    body = _make_body(b"hello world")
    chunks = list(body.iter_bytes())
    assert b"".join(chunks) == b"hello world"

def test_save_to(tmp_path):
    body = _make_body(b"file contents here")
    dest = tmp_path / "output.bin"
    body.save_to(dest)
    assert dest.read_bytes() == b"file contents here"

def test_read_twice_raises():
    body = _make_body(b"data")
    body.read()
    with pytest.raises(RuntimeError, match="already consumed"):
        body.read()

def test_iter_twice_raises():
    body = _make_body(b"data")
    list(body.iter_bytes())
    with pytest.raises(RuntimeError, match="already consumed"):
        list(body.iter_bytes())
