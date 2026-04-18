from __future__ import annotations
import pathlib
from typing import Iterator


class StreamingBody:
    def __init__(self, iterator: Iterator[bytes]) -> None:
        self._iterator = iterator
        self._consumed = False

    def _check_and_mark(self) -> None:
        if self._consumed:
            raise RuntimeError("StreamingBody already consumed")
        self._consumed = True

    def iter_bytes(self) -> Iterator[bytes]:
        self._check_and_mark()
        yield from self._iterator

    def read(self) -> bytes:
        self._check_and_mark()
        return b"".join(self._iterator)

    def save_to(self, path: str | pathlib.Path) -> None:
        self._check_and_mark()
        with open(path, "wb") as f:
            for chunk in self._iterator:
                f.write(chunk)
