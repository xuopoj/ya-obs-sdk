import pytest
from ya_obs._streaming import StreamingBody

def _make_body(data: bytes, total_bytes: int | None = None) -> StreamingBody:
    def _iter():
        chunk_size = 4
        for i in range(0, len(data), chunk_size):
            yield data[i:i + chunk_size]
    return StreamingBody(iterator=_iter(), total_bytes=total_bytes)

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


def test_iter_bytes_progress_reports_cumulative_bytes():
    body = _make_body(b"hello world", total_bytes=11)
    events = []
    list(body.iter_bytes(on_progress=events.append))
    assert [e.bytes_transferred for e in events] == [4, 8, 11]
    assert all(e.total_bytes == 11 for e in events)


def test_read_progress_reports_total_at_end():
    body = _make_body(b"hello world", total_bytes=11)
    events = []
    body.read(on_progress=events.append)
    assert events[-1].bytes_transferred == 11
    assert events[-1].total_bytes == 11


def test_save_to_progress(tmp_path):
    body = _make_body(b"file contents here", total_bytes=18)
    events = []
    body.save_to(tmp_path / "out.bin", on_progress=events.append)
    assert events[-1].bytes_transferred == 18
    assert (tmp_path / "out.bin").read_bytes() == b"file contents here"


def test_progress_without_total_bytes_reports_zero_total():
    body = _make_body(b"abcd")
    events = []
    list(body.iter_bytes(on_progress=events.append))
    assert events[-1].bytes_transferred == 4
    assert events[-1].total_bytes == 0
