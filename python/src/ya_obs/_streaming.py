from __future__ import annotations
import pathlib
from typing import Callable, Iterator

from ._models import ProgressEvent


class StreamingBody:
    def __init__(self, iterator: Iterator[bytes], total_bytes: int | None = None) -> None:
        self._iterator = iterator
        self._consumed = False
        self.total_bytes = total_bytes

    def _check_and_mark(self) -> None:
        if self._consumed:
            raise RuntimeError("StreamingBody already consumed")
        self._consumed = True

    def _iter_with_progress(
        self,
        on_progress: Callable[[ProgressEvent], None] | None,
    ) -> Iterator[bytes]:
        if on_progress is None:
            yield from self._iterator
            return
        total = self.total_bytes or 0
        done = 0
        for chunk in self._iterator:
            done += len(chunk)
            yield chunk
            on_progress(ProgressEvent(bytes_transferred=done, total_bytes=total))

    def iter_bytes(
        self,
        on_progress: Callable[[ProgressEvent], None] | None = None,
    ) -> Iterator[bytes]:
        self._check_and_mark()
        yield from self._iter_with_progress(on_progress)

    def read(
        self,
        on_progress: Callable[[ProgressEvent], None] | None = None,
    ) -> bytes:
        self._check_and_mark()
        return b"".join(self._iter_with_progress(on_progress))

    def save_to(
        self,
        path: str | pathlib.Path,
        on_progress: Callable[[ProgressEvent], None] | None = None,
    ) -> None:
        self._check_and_mark()
        with open(path, "wb") as f:
            for chunk in self._iter_with_progress(on_progress):
                f.write(chunk)
