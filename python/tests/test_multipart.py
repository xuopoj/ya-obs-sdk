import math
from ya_obs._multipart import compute_part_size, split_into_parts

_8MB = 8 * 1024 * 1024

def test_small_file_uses_8mb_parts():
    assert compute_part_size(total_size=50 * 1024 * 1024) == _8MB

def test_large_file_adapts_part_size():
    size = 100 * 1024 * 1024 * 1024
    part_size = compute_part_size(total_size=size)
    assert part_size > _8MB
    assert math.ceil(size / part_size) <= 9000

def test_split_into_parts_count():
    data = b"x" * (20 * 1024 * 1024)
    parts = split_into_parts(data, part_size=_8MB)
    assert len(parts) == 3
    assert len(parts[0][1]) == _8MB
    assert len(parts[2][1]) == 4 * 1024 * 1024

def test_split_into_parts_numbering():
    data = b"y" * (10 * 1024 * 1024)
    parts = split_into_parts(data, part_size=_8MB)
    assert parts[0][0] == 1
    assert parts[1][0] == 2

def test_split_into_parts_reconstructs():
    data = b"hello world " * 1000000
    parts = split_into_parts(data, part_size=_8MB)
    reconstructed = b"".join(chunk for _, chunk in parts)
    assert reconstructed == data
